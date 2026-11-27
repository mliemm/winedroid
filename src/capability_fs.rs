use std::ffi::{CStr, CString};
use std::fs::File;
use std::io::{self, Read, Write};
use std::os::fd::{AsFd, OwnedFd};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[cfg(not(target_os = "emscripten"))]
use rustix::fs::RenameFlags;
use rustix::fs::{self, AtFlags, Dir, FileType, Mode, OFlags, Stat};

static TEMPORARY_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

const DIRECTORY_FLAGS: OFlags = OFlags::RDONLY
    .union(OFlags::DIRECTORY)
    .union(OFlags::NOFOLLOW)
    .union(OFlags::CLOEXEC);
const FILE_MODE: Mode = Mode::RUSR
    .union(Mode::WUSR)
    .union(Mode::RGRP)
    .union(Mode::WGRP)
    .union(Mode::ROTH)
    .union(Mode::WOTH);
const DIRECTORY_MODE: Mode = Mode::RWXU.union(Mode::RWXG).union(Mode::RWXO);

#[derive(Debug, Clone)]
pub struct Directory {
    fd: Arc<OwnedFd>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetadataKind {
    File,
    Directory,
    Symlink,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Metadata {
    pub kind: MetadataKind,
    pub len: u64,
    pub modified_millis: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VolumeStats {
    pub block_size: u64,
    pub total_blocks: u64,
    pub free_blocks: u64,
    pub available_blocks: u64,
}

#[cfg(target_os = "emscripten")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileIdentity {
    pub device: u64,
    pub inode: u64,
}

struct Parent {
    directory: OwnedFd,
    leaf: CString,
}

impl Directory {
    pub fn open_ambient(path: &Path) -> io::Result<Self> {
        let fd = fs::open(path, DIRECTORY_FLAGS, Mode::empty()).map_err(io::Error::from)?;
        Ok(Self { fd: Arc::new(fd) })
    }

    pub fn open_child(&self, name: &CStr) -> io::Result<Self> {
        validate_leaf_component(name)?;
        let fd = open_directory(self.fd.as_ref(), name)?;
        Ok(Self { fd: Arc::new(fd) })
    }

    #[cfg(target_os = "emscripten")]
    pub fn identity(&self) -> io::Result<FileIdentity> {
        fs::fstat(self.fd.as_ref())
            .map(file_identity_from_stat)
            .map_err(io::Error::from)
    }

    #[cfg(target_os = "emscripten")]
    pub fn child_identity(&self, name: &CStr) -> io::Result<FileIdentity> {
        validate_leaf_component(name)?;
        fs::statat(self.fd.as_ref(), name, AtFlags::SYMLINK_NOFOLLOW)
            .map(file_identity_from_stat)
            .map_err(io::Error::from)
    }

    pub fn volume_stats(&self, components: &[CString]) -> io::Result<VolumeStats> {
        // cite: libcore Os.java#statvfs
        let stat = match self.target(components)? {
            Target::Directory(fd) => fs::fstatvfs(&fd),
            Target::Leaf(parent) => fs::fstatvfs(&parent.directory),
        }
        .map_err(io::Error::from)?;

        Ok(VolumeStats {
            block_size: stat.f_frsize,
            total_blocks: stat.f_blocks,
            free_blocks: stat.f_bfree,
            available_blocks: stat.f_bavail,
        })
    }

    pub fn metadata(&self, components: &[CString]) -> io::Result<Metadata> {
        match self.target(components)? {
            Target::Directory(fd) => fs::fstat(&fd)
                .map(metadata_from_stat)
                .map_err(io::Error::from),
            Target::Leaf(parent) => {
                fs::statat(&parent.directory, &parent.leaf, AtFlags::SYMLINK_NOFOLLOW)
                    .map(metadata_from_stat)
                    .map_err(io::Error::from)
            }
        }
    }

    pub fn can_access(&self, components: &[CString], write: bool) -> io::Result<bool> {
        let result = match self.target(components)? {
            Target::Directory(fd) => directory_access(&fd, write),
            Target::Leaf(parent) => {
                let stat =
                    match fs::statat(&parent.directory, &parent.leaf, AtFlags::SYMLINK_NOFOLLOW) {
                        Ok(stat) => stat,
                        Err(_) => return Ok(false),
                    };
                match FileType::from_raw_mode(stat.st_mode) {
                    FileType::Symlink => return Ok(false),
                    FileType::Directory => open_directory(&parent.directory, &parent.leaf)
                        .and_then(|directory| directory_access(&directory, write)),
                    _ => fs::openat(
                        &parent.directory,
                        &parent.leaf,
                        if write {
                            OFlags::WRONLY | OFlags::NONBLOCK | OFlags::NOFOLLOW | OFlags::CLOEXEC
                        } else {
                            OFlags::RDONLY | OFlags::NONBLOCK | OFlags::NOFOLLOW | OFlags::CLOEXEC
                        },
                        Mode::empty(),
                    )
                    .map(|_| ())
                    .map_err(io::Error::from),
                }
            }
        };
        match result {
            Ok(()) => Ok(true),
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::PermissionDenied
                        | io::ErrorKind::NotFound
                        | io::ErrorKind::NotADirectory
                ) =>
            {
                Ok(false)
            }
            Err(error) => Err(error),
        }
    }

    pub fn open_read(&self, components: &[CString]) -> io::Result<File> {
        let parent = self.parent(components)?;
        let fd = fs::openat(
            &parent.directory,
            &parent.leaf,
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(io::Error::from)?;
        Ok(File::from(fd))
    }

    pub fn read(&self, components: &[CString]) -> io::Result<Vec<u8>> {
        let mut file = self.open_read(components)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        Ok(bytes)
    }

    pub fn open_truncate(&self, components: &[CString]) -> io::Result<File> {
        self.open_write(components, OFlags::WRONLY | OFlags::CREATE | OFlags::TRUNC)
    }

    pub fn open_append(&self, components: &[CString]) -> io::Result<File> {
        self.open_write(components, OFlags::WRONLY | OFlags::CREATE | OFlags::APPEND)
    }

    pub fn create_new(&self, components: &[CString]) -> io::Result<File> {
        self.open_write(components, OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL)
    }

    pub fn mkdir(&self, components: &[CString]) -> io::Result<()> {
        let parent = self.parent(components)?;
        fs::mkdirat(&parent.directory, &parent.leaf, DIRECTORY_MODE).map_err(io::Error::from)
    }

    pub fn mkdir_all(&self, components: &[CString]) -> io::Result<bool> {
        let mut directories = vec![duplicate(self.fd.as_ref())?];
        let mut created = false;
        for component in components {
            validate_path_component(component)?;
            if component.as_bytes() == b".." {
                if directories.len() == 1 {
                    return Err(path_escape_error());
                }
                directories.pop();
                continue;
            }

            let parent = directories.last().expect("root capability is retained");
            let child = match open_directory(parent, component) {
                Ok(child) => child,
                Err(error) if error.kind() == io::ErrorKind::NotFound => {
                    match fs::mkdirat(parent, component, DIRECTORY_MODE) {
                        Ok(()) => created = true,
                        Err(rustix::io::Errno::EXIST) => {}
                        Err(error) => return Err(io::Error::from(error)),
                    }
                    open_directory(parent, component)?
                }
                Err(error) => return Err(error),
            };
            directories.push(child);
        }

        Ok(created)
    }

    pub fn clear_write_permission(&self, components: &[CString]) -> io::Result<()> {
        let without_write = |stat: Stat| {
            Mode::from_raw_mode(stat.st_mode as _) & !(Mode::WUSR | Mode::WGRP | Mode::WOTH)
        };
        match self.target(components)? {
            Target::Directory(fd) => {
                let stat = fs::fstat(&fd).map_err(io::Error::from)?;
                fs::fchmod(&fd, without_write(stat)).map_err(io::Error::from)
            }
            Target::Leaf(parent) => {
                let stat = fs::statat(&parent.directory, &parent.leaf, AtFlags::SYMLINK_NOFOLLOW)
                    .map_err(io::Error::from)?;
                if FileType::from_raw_mode(stat.st_mode as _) == FileType::Symlink {
                    return Err(io::Error::from(io::ErrorKind::PermissionDenied));
                }
                fs::chmodat(
                    &parent.directory,
                    &parent.leaf,
                    without_write(stat),
                    AtFlags::empty(),
                )
                .map_err(io::Error::from)
            }
        }
    }

    pub fn remove(&self, components: &[CString]) -> io::Result<()> {
        let parent = self.parent(components)?;
        let stat = fs::statat(&parent.directory, &parent.leaf, AtFlags::SYMLINK_NOFOLLOW)
            .map_err(io::Error::from)?;
        let flags = if FileType::from_raw_mode(stat.st_mode) == FileType::Directory {
            AtFlags::REMOVEDIR
        } else {
            AtFlags::empty()
        };
        fs::unlinkat(&parent.directory, &parent.leaf, flags).map_err(io::Error::from)
    }

    pub fn rename(
        &self,
        from_components: &[CString],
        to: &Directory,
        to_components: &[CString],
    ) -> io::Result<()> {
        let from = self.parent(from_components)?;
        let to = to.parent(to_components)?;
        fs::renameat(&from.directory, &from.leaf, &to.directory, &to.leaf).map_err(io::Error::from)
    }

    pub fn rename_noreplace(
        &self,
        from_components: &[CString],
        to: &Directory,
        to_components: &[CString],
    ) -> io::Result<()> {
        let from = self.parent(from_components)?;
        let to = to.parent(to_components)?;
        #[cfg(not(target_os = "emscripten"))]
        {
            fs::renameat_with(
                &from.directory,
                &from.leaf,
                &to.directory,
                &to.leaf,
                RenameFlags::NOREPLACE,
            )
            .map_err(io::Error::from)
        }
        #[cfg(target_os = "emscripten")]
        {
            match fs::statat(&to.directory, &to.leaf, AtFlags::SYMLINK_NOFOLLOW) {
                Ok(_) => Err(io::Error::from(io::ErrorKind::AlreadyExists)),
                Err(rustix::io::Errno::NOENT) => {
                    fs::renameat(&from.directory, &from.leaf, &to.directory, &to.leaf)
                        .map_err(io::Error::from)
                }
                Err(error) => Err(io::Error::from(error)),
            }
        }
    }

    pub fn read_dir_names(&self, components: &[CString]) -> io::Result<Vec<String>> {
        let directory = self.open_directory_path(components)?;
        // cite: bionic fcntl.cpp#F_DUPFD_CLOEXEC
        // cite: bionic open.cpp#openat
        let directory = fs::openat(&directory, c".", DIRECTORY_FLAGS, Mode::empty())
            .map_err(io::Error::from)?;
        let mut entries = Dir::new(directory).map_err(io::Error::from)?;
        let mut names = Vec::new();
        for entry in &mut entries {
            let entry = entry.map_err(io::Error::from)?;
            let bytes = entry.file_name().to_bytes();
            if matches!(bytes, b"." | b"..") {
                continue;
            }
            if let Ok(name) = std::str::from_utf8(bytes) {
                names.push(name.to_owned());
            }
        }
        Ok(names)
    }

    pub fn atomic_replace(&self, components: &[CString], bytes: &[u8]) -> io::Result<()> {
        let parent = self.parent(components)?;
        let temporary = create_temporary(&parent.directory, &parent.leaf)?;
        let temporary_name = temporary.0;
        let mut file = temporary.1;
        let result = (|| {
            file.write_all(bytes)?;
            file.sync_all()?;
            drop(file);
            fs::renameat(
                &parent.directory,
                &temporary_name,
                &parent.directory,
                &parent.leaf,
            )
            .map_err(io::Error::from)
        })();
        if result.is_err() {
            let _ = fs::unlinkat(&parent.directory, &temporary_name, AtFlags::empty());
        }
        result
    }

    pub fn as_fd(&self) -> std::os::fd::BorrowedFd<'_> {
        self.fd.as_fd()
    }

    pub fn parent_and_leaf(
        &self,
        components: &[CString],
    ) -> io::Result<(Directory, Option<CString>)> {
        match self.target(components)? {
            Target::Leaf(parent) => Ok((
                Directory {
                    fd: Arc::new(parent.directory),
                },
                Some(parent.leaf),
            )),
            Target::Directory(fd) => Ok((Directory { fd: Arc::new(fd) }, None)),
        }
    }

    pub fn open_leaf(&self, name: &CStr, flags: OFlags, mode: Mode) -> io::Result<OwnedFd> {
        validate_leaf_component(name)?;
        fs::openat(self.fd.as_ref(), name, flags, mode).map_err(io::Error::from)
    }

    pub fn unlink_leaf(&self, name: &CStr) -> io::Result<()> {
        validate_leaf_component(name)?;
        fs::unlinkat(self.fd.as_ref(), name, AtFlags::empty()).map_err(io::Error::from)
    }

    pub fn stat_leaf(&self, name: &CStr) -> io::Result<Stat> {
        validate_leaf_component(name)?;
        fs::statat(self.fd.as_ref(), name, AtFlags::SYMLINK_NOFOLLOW).map_err(io::Error::from)
    }

    pub fn sync(&self) -> io::Result<()> {
        fs::fsync(self.fd.as_ref()).map_err(io::Error::from)
    }

    pub fn parent_directory_and_leaf(
        &self,
        components: &[CString],
    ) -> io::Result<(Directory, CString)> {
        let parent = self.parent(components)?;
        Ok((
            Directory {
                fd: Arc::new(parent.directory),
            },
            parent.leaf,
        ))
    }

    fn open_write(&self, components: &[CString], flags: OFlags) -> io::Result<File> {
        let parent = self.parent(components)?;
        let fd = fs::openat(
            &parent.directory,
            &parent.leaf,
            flags | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            FILE_MODE,
        )
        .map_err(io::Error::from)?;
        Ok(File::from(fd))
    }

    fn open_directory_path(&self, components: &[CString]) -> io::Result<OwnedFd> {
        let mut directories = vec![duplicate(self.fd.as_ref())?];
        for component in components {
            validate_path_component(component)?;
            if component.as_bytes() == b".." {
                if directories.len() == 1 {
                    return Err(path_escape_error());
                }
                directories.pop();
                continue;
            }
            let parent = directories.last().expect("root capability is retained");
            directories.push(open_directory(parent, component)?);
        }
        Ok(directories.pop().expect("root capability is retained"))
    }

    fn parent(&self, components: &[CString]) -> io::Result<Parent> {
        match self.target(components)? {
            Target::Leaf(parent) => Ok(parent),
            Target::Directory(_) => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "path does not name a removable leaf",
            )),
        }
    }

    fn target(&self, components: &[CString]) -> io::Result<Target> {
        let mut directories = vec![duplicate(self.fd.as_ref())?];
        for (index, component) in components.iter().enumerate() {
            validate_path_component(component)?;
            if component.as_bytes() == b".." {
                if directories.len() == 1 {
                    return Err(path_escape_error());
                }
                directories.pop();
                continue;
            }

            if index + 1 == components.len() {
                return Ok(Target::Leaf(Parent {
                    directory: directories.pop().expect("root capability is retained"),
                    leaf: component.clone(),
                }));
            }

            let parent = directories.last().expect("root capability is retained");
            directories.push(open_directory(parent, component)?);
        }

        Ok(Target::Directory(
            directories.pop().expect("root capability is retained"),
        ))
    }
}

enum Target {
    Directory(OwnedFd),
    Leaf(Parent),
}

fn open_directory(parent: impl AsFd, name: &CStr) -> io::Result<OwnedFd> {
    validate_leaf_component(name)?;
    fs::openat(parent, name, DIRECTORY_FLAGS, Mode::empty()).map_err(io::Error::from)
}

fn validate_path_component(component: &CStr) -> io::Result<()> {
    if component.to_bytes() == b".." {
        return Ok(());
    }
    validate_leaf_component(component)
}

fn validate_leaf_component(component: &CStr) -> io::Result<()> {
    let bytes = component.to_bytes();
    if bytes.is_empty() || matches!(bytes, b"." | b"..") || bytes.contains(&b'/') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "directory capability requires one relative path component",
        ));
    }
    Ok(())
}

fn directory_access(directory: impl AsFd, write: bool) -> io::Result<()> {
    fs::accessat(
        directory,
        c".",
        if write {
            rustix::fs::Access::WRITE_OK
        } else {
            rustix::fs::Access::READ_OK
        },
        AtFlags::empty(),
    )
    .map_err(io::Error::from)
}

fn duplicate(fd: impl AsFd) -> io::Result<OwnedFd> {
    rustix::io::fcntl_dupfd_cloexec(fd, 0).map_err(io::Error::from)
}

fn metadata_from_stat(stat: Stat) -> Metadata {
    let kind = match FileType::from_raw_mode(stat.st_mode) {
        FileType::RegularFile => MetadataKind::File,
        FileType::Directory => MetadataKind::Directory,
        FileType::Symlink => MetadataKind::Symlink,
        _ => MetadataKind::Other,
    };
    let modified_millis = i128::from(stat.st_mtime)
        .saturating_mul(1000)
        .saturating_add(i128::from(stat.st_mtime_nsec) / 1_000_000)
        .clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64;
    Metadata {
        kind,
        len: stat.st_size.max(0) as u64,
        modified_millis,
    }
}

#[cfg(target_os = "emscripten")]
fn file_identity_from_stat(stat: Stat) -> FileIdentity {
    FileIdentity {
        device: stat.st_dev as u64,
        inode: stat.st_ino as u64,
    }
}

fn create_temporary(parent: &OwnedFd, destination: &CStr) -> io::Result<(CString, File)> {
    for _ in 0..128 {
        let sequence = TEMPORARY_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let name = CString::new(format!(
            ".{}.winedroid-{}-{sequence}",
            destination.to_string_lossy(),
            std::process::id()
        ))
        .expect("generated temporary filename has no NUL");
        match fs::openat(
            parent,
            &name,
            OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            FILE_MODE,
        ) {
            Ok(fd) => return Ok((name, File::from(fd))),
            Err(rustix::io::Errno::EXIST) => {}
            Err(error) => return Err(io::Error::from(error)),
        }
    }

    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "failed to allocate a unique temporary filename",
    ))
}

fn path_escape_error() -> io::Error {
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        "relative path escapes its directory capability",
    )
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::symlink;
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;

    static FIXTURE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn openat_truncate_stays_in_held_parent_after_path_swap() {
        let fixture = Fixture::new("truncate_parent_swap");
        fs::create_dir(fixture.root.join("inside")).unwrap();
        fs::write(fixture.root.join("inside/value"), b"inside").unwrap();
        fs::create_dir(fixture.root.join("outside")).unwrap();
        fs::write(fixture.root.join("outside/value"), b"outside").unwrap();
        let root = Directory::open_ambient(&fixture.root).unwrap();
        let held = root.open_child(cstr("inside")).unwrap();

        fs::rename(fixture.root.join("inside"), fixture.root.join("moved")).unwrap();
        symlink(fixture.root.join("outside"), fixture.root.join("inside")).unwrap();
        held.open_truncate(&[cstring("value")]).unwrap();

        assert_eq!(fs::read(fixture.root.join("moved/value")).unwrap(), b"");
        assert_eq!(
            fs::read(fixture.root.join("outside/value")).unwrap(),
            b"outside"
        );
    }

    #[test]
    fn openat_truncate_rejects_a_final_symlink() {
        let fixture = Fixture::new("truncate_leaf_symlink");
        fs::create_dir(fixture.root.join("inside")).unwrap();
        fs::create_dir(fixture.root.join("outside")).unwrap();
        fs::write(fixture.root.join("outside/value"), b"outside").unwrap();
        symlink(
            fixture.root.join("outside/value"),
            fixture.root.join("inside/value"),
        )
        .unwrap();
        let root = Directory::open_ambient(&fixture.root).unwrap();

        assert!(root
            .open_truncate(&[cstring("inside"), cstring("value")])
            .is_err());
        assert_eq!(
            fs::read(fixture.root.join("outside/value")).unwrap(),
            b"outside"
        );
    }

    #[test]
    fn read_dir_names_on_the_capability_root_is_repeatable() {
        // cite: bionic fcntl.cpp#F_DUPFD_CLOEXEC
        // cite: bionic open.cpp#openat
        let fixture = Fixture::new("read_dir_names_repeatable");
        fs::create_dir(fixture.root.join("alpha")).unwrap();
        fs::create_dir(fixture.root.join("beta")).unwrap();
        let root = Directory::open_ambient(&fixture.root).unwrap();

        let mut first = root.read_dir_names(&[]).unwrap();
        first.sort();
        assert_eq!(first, vec!["alpha".to_owned(), "beta".to_owned()]);

        let mut nested = root.read_dir_names(&[cstring("alpha")]).unwrap();
        nested.sort();
        assert!(nested.is_empty());

        let mut second = root.read_dir_names(&[]).unwrap();
        second.sort();
        assert_eq!(
            second,
            vec!["alpha".to_owned(), "beta".to_owned()],
            "a second listing of the same root must see the same entries as the first"
        );
    }

    #[test]
    fn capability_rejects_slash_and_absolute_component_injection() {
        let fixture = Fixture::new("component_injection");
        fs::create_dir(fixture.root.join("inside")).unwrap();
        fs::create_dir(fixture.root.join("outside")).unwrap();
        fs::write(fixture.root.join("outside/value"), b"outside").unwrap();
        let held = Directory::open_ambient(&fixture.root.join("inside")).unwrap();

        assert!(held.open_truncate(&[cstring("../outside/value")]).is_err());
        let absolute = fixture.root.join("absolute");
        assert!(held
            .open_truncate(&[cstring(absolute.to_str().unwrap())])
            .is_err());
        let raw_escape = cstring("../outside/raw");
        assert!(held
            .open_leaf(&raw_escape, OFlags::WRONLY | OFlags::CREATE, FILE_MODE,)
            .is_err());

        assert_eq!(
            fs::read(fixture.root.join("outside/value")).unwrap(),
            b"outside"
        );
        assert!(!fixture.root.join("outside/raw").exists());
        assert!(!absolute.exists());
    }

    #[test]
    fn mkdirat_and_unlinkat_stay_in_held_parent() {
        let fixture = Fixture::new("mkdir_unlink_parent_swap");
        fs::create_dir(fixture.root.join("inside")).unwrap();
        fs::write(fixture.root.join("inside/remove"), b"inside").unwrap();
        fs::create_dir(fixture.root.join("outside")).unwrap();
        fs::write(fixture.root.join("outside/remove"), b"outside").unwrap();
        let root = Directory::open_ambient(&fixture.root).unwrap();
        let held = root.open_child(cstr("inside")).unwrap();

        fs::rename(fixture.root.join("inside"), fixture.root.join("moved")).unwrap();
        symlink(fixture.root.join("outside"), fixture.root.join("inside")).unwrap();
        held.mkdir(&[cstring("created")]).unwrap();
        held.remove(&[cstring("remove")]).unwrap();
        held.remove(&[cstring("created")]).unwrap();

        assert!(!fixture.root.join("moved/remove").exists());
        assert!(!fixture.root.join("moved/created").exists());
        assert_eq!(
            fs::read(fixture.root.join("outside/remove")).unwrap(),
            b"outside"
        );
    }

    #[test]
    fn renameat_uses_both_held_parents_after_path_swaps() {
        let fixture = Fixture::new("rename_parent_swap");
        for name in [
            "source",
            "destination",
            "outside_source",
            "outside_destination",
        ] {
            fs::create_dir(fixture.root.join(name)).unwrap();
        }
        fs::write(fixture.root.join("source/value"), b"inside").unwrap();
        fs::write(fixture.root.join("outside_source/value"), b"outside").unwrap();
        let root = Directory::open_ambient(&fixture.root).unwrap();
        let source = root.open_child(cstr("source")).unwrap();
        let destination = root.open_child(cstr("destination")).unwrap();

        fs::rename(
            fixture.root.join("source"),
            fixture.root.join("moved_source"),
        )
        .unwrap();
        fs::rename(
            fixture.root.join("destination"),
            fixture.root.join("moved_destination"),
        )
        .unwrap();
        symlink(
            fixture.root.join("outside_source"),
            fixture.root.join("source"),
        )
        .unwrap();
        symlink(
            fixture.root.join("outside_destination"),
            fixture.root.join("destination"),
        )
        .unwrap();
        source
            .rename(&[cstring("value")], &destination, &[cstring("renamed")])
            .unwrap();

        assert!(!fixture.root.join("moved_source/value").exists());
        assert_eq!(
            fs::read(fixture.root.join("moved_destination/renamed")).unwrap(),
            b"inside"
        );
        assert_eq!(
            fs::read(fixture.root.join("outside_source/value")).unwrap(),
            b"outside"
        );
        assert!(!fixture.root.join("outside_destination/renamed").exists());
    }

    #[test]
    fn renameat_noreplace_preserves_a_concurrent_destination() {
        let fixture = Fixture::new("rename_noreplace");
        fs::write(fixture.root.join("source"), b"source").unwrap();
        fs::write(fixture.root.join("destination"), b"destination").unwrap();
        let root = Directory::open_ambient(&fixture.root).unwrap();

        let error = root
            .rename_noreplace(&[cstring("source")], &root, &[cstring("destination")])
            .unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(fs::read(fixture.root.join("source")).unwrap(), b"source");
        assert_eq!(
            fs::read(fixture.root.join("destination")).unwrap(),
            b"destination"
        );
    }

    #[test]
    fn atomic_replace_and_mkdir_all_stay_in_held_root_after_path_swap() {
        let fixture = Fixture::new("atomic_replace_root_swap");
        fs::create_dir_all(fixture.root.join("inside/preferences")).unwrap();
        fs::create_dir_all(fixture.root.join("outside/preferences")).unwrap();
        fs::write(fixture.root.join("outside/preferences/store"), b"outside").unwrap();
        let inside = fixture.root.join("inside");
        let held = Directory::open_ambient(&inside).unwrap();

        fs::rename(&inside, fixture.root.join("moved")).unwrap();
        symlink(fixture.root.join("outside"), &inside).unwrap();
        held.mkdir_all(&[cstring("created"), cstring("nested")])
            .unwrap();
        held.atomic_replace(&[cstring("preferences"), cstring("store")], b"inside")
            .unwrap();

        assert!(fixture.root.join("moved/created/nested").is_dir());
        assert_eq!(
            fs::read(fixture.root.join("moved/preferences/store")).unwrap(),
            b"inside"
        );
        assert!(!fixture.root.join("outside/created").exists());
        assert_eq!(
            fs::read(fixture.root.join("outside/preferences/store")).unwrap(),
            b"outside"
        );
    }

    fn cstr(value: &str) -> &CStr {
        CStr::from_bytes_with_nul(match value {
            "inside" => b"inside\0",
            "source" => b"source\0",
            "destination" => b"destination\0",
            _ => unreachable!(),
        })
        .unwrap()
    }

    fn cstring(value: &str) -> CString {
        CString::new(value).unwrap()
    }

    struct Fixture {
        root: std::path::PathBuf,
    }

    impl Fixture {
        fn new(name: &str) -> Self {
            let sequence = FIXTURE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir().join(format!(
                "winedroid-capability-fs-{name}-{}-{sequence}",
                std::process::id()
            ));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&root).unwrap();
            Self { root }
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}
