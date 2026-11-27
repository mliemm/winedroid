use std::collections::{HashMap, VecDeque};
use std::ffi::{CStr, CString};
use std::fs::File;
use std::io;
use std::ops::{Deref, DerefMut};
use std::os::fd::{AsFd, BorrowedFd};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, Weak};

use rustix::fs::{AtFlags, FileType, Stat};

// cite: bionic inotify.h#IN_CLOSE_WRITE, IN_CREATE, IN_IGNORED, IN_ISDIR, IN_ALL_EVENTS
pub const IN_CLOSE_WRITE: u32 = 0x0000_0008;
pub const IN_CREATE: u32 = 0x0000_0100;
pub const IN_IGNORED: u32 = 0x0000_8000;
pub const IN_ISDIR: u32 = 0x4000_0000;
const IN_ALL_EVENTS: u32 = 0x0000_0fff;
pub const WIRED_EVENTS: u32 = IN_CREATE | IN_CLOSE_WRITE;

static INSTANCES: Mutex<Vec<Weak<Instance>>> = Mutex::new(Vec::new());
static ACTIVE_WATCHES: AtomicUsize = AtomicUsize::new(0);
static OPEN_DENTRIES: Mutex<Vec<Weak<Mutex<Dentry>>>> = Mutex::new(Vec::new());

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InodeKey {
    device: u64,
    inode: u64,
}

impl InodeKey {
    pub fn of(fd: impl AsFd) -> io::Result<Self> {
        rustix::fs::fstat(fd)
            .map(Self::from_stat)
            .map_err(io::Error::from)
    }

    fn from_stat(stat: Stat) -> Self {
        Self {
            device: stat.st_dev as u64,
            inode: stat.st_ino as u64,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub wd: i32,
    pub mask: u32,
    pub name: Option<CString>,
}

#[derive(Default)]
struct InstanceState {
    last_wd: i32,
    watches: HashMap<i32, (InodeKey, u32)>,
    queue: VecDeque<Event>,
}

pub struct Instance {
    state: Mutex<InstanceState>,
}

impl Instance {
    pub fn new() -> Arc<Self> {
        let instance = Arc::new(Self {
            state: Mutex::new(InstanceState::default()),
        });
        let mut instances = INSTANCES.lock().expect("fs_notify instances");
        instances.retain(|weak| weak.strong_count() > 0);
        instances.push(Arc::downgrade(&instance));
        instance
    }

    pub fn add_watch(&self, key: InodeKey, mask: u32) -> Option<i32> {
        if mask & IN_ALL_EVENTS == 0 {
            return None;
        }
        let mut state = self.state();
        let existing = state
            .watches
            .iter()
            .find(|(_, (watched, _))| *watched == key)
            .map(|(wd, _)| *wd);
        let wd = match existing {
            Some(wd) => wd,
            None => {
                state.last_wd += 1;
                ACTIVE_WATCHES.fetch_add(1, Ordering::SeqCst);
                state.last_wd
            }
        };
        state.watches.insert(wd, (key, mask));
        Some(wd)
    }

    pub fn rm_watch(&self, wd: i32) -> bool {
        let mut state = self.state();
        if state.watches.remove(&wd).is_none() {
            return false;
        }
        ACTIVE_WATCHES.fetch_sub(1, Ordering::SeqCst);
        state.queue.push_back(Event {
            wd,
            mask: IN_IGNORED,
            name: None,
        });
        true
    }

    pub fn has_events(&self) -> bool {
        !self.state().queue.is_empty()
    }

    pub fn take_events(&self) -> Vec<Event> {
        self.state().queue.drain(..).collect()
    }

    fn state(&self) -> std::sync::MutexGuard<'_, InstanceState> {
        self.state.lock().expect("fs_notify instance state")
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        let watches = self
            .state
            .get_mut()
            .map(|state| state.watches.len())
            .unwrap_or(0);
        ACTIVE_WATCHES.fetch_sub(watches, Ordering::SeqCst);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Dentry {
    parent: InodeKey,
    leaf: CString,
    file: InodeKey,
}

#[derive(Debug)]
pub struct CloseNote {
    dentry: Arc<Mutex<Dentry>>,
}

impl Drop for CloseNote {
    fn drop(&mut self) {
        let dentry = self.dentry.lock().expect("fs_notify close note").clone();
        queue_event(dentry.parent, IN_CLOSE_WRITE, Some(&dentry.leaf));
    }
}

#[derive(Default)]
pub struct Rename {
    moves: Vec<(Dentry, InodeKey, CString)>,
    replaced_directory: Option<InodeKey>,
}

impl Rename {
    pub fn prepare(
        from_parent: BorrowedFd<'_>,
        from_leaf: &CStr,
        to_parent: BorrowedFd<'_>,
        to_leaf: &CStr,
        exchange: bool,
    ) -> Self {
        if !watching() && open_dentries().is_empty() {
            return Self::default();
        }
        let (Ok(from_key), Ok(to_key)) = (InodeKey::of(from_parent), InodeKey::of(to_parent))
        else {
            return Self::default();
        };
        let Ok(source) = rustix::fs::statat(from_parent, from_leaf, AtFlags::SYMLINK_NOFOLLOW)
        else {
            return Self::default();
        };
        let source = InodeKey::from_stat(source);
        let target = rustix::fs::statat(to_parent, to_leaf, AtFlags::SYMLINK_NOFOLLOW).ok();
        let target_key = target.map(InodeKey::from_stat);
        if target_key == Some(source) {
            return Self::default();
        }
        let mut moves = vec![(
            Dentry {
                parent: from_key,
                leaf: from_leaf.to_owned(),
                file: source,
            },
            to_key,
            to_leaf.to_owned(),
        )];
        if let (true, Some(target)) = (exchange, target_key) {
            moves.push((
                Dentry {
                    parent: to_key,
                    leaf: to_leaf.to_owned(),
                    file: target,
                },
                from_key,
                from_leaf.to_owned(),
            ));
        }
        let replaced_directory = target
            .filter(|stat| {
                !exchange
                    && watching()
                    && FileType::from_raw_mode(stat.st_mode) == FileType::Directory
            })
            .map(InodeKey::from_stat);
        Self {
            moves,
            replaced_directory,
        }
    }

    pub fn finish(self) {
        for dentry in open_dentries() {
            let mut dentry = dentry.lock().expect("fs_notify close note");
            let moved = self.moves.iter().find(|(from, _, _)| *from == *dentry);
            if let Some((_, parent, leaf)) = moved {
                dentry.parent = *parent;
                dentry.leaf = leaf.clone();
            }
        }
        if let Some(key) = self.replaced_directory {
            removed_directory(key);
        }
    }
}

#[derive(Debug)]
pub struct NotifiedFile {
    file: File,
    _close: Option<CloseNote>,
}

impl NotifiedFile {
    pub fn new(file: File, close: Option<CloseNote>) -> Self {
        Self {
            file,
            _close: close,
        }
    }
}

impl Deref for NotifiedFile {
    type Target = File;

    fn deref(&self) -> &File {
        &self.file
    }
}

impl DerefMut for NotifiedFile {
    fn deref_mut(&mut self) -> &mut File {
        &mut self.file
    }
}

pub fn created(parent: BorrowedFd<'_>, leaf: &CStr, is_directory: bool) {
    if !watching() {
        return;
    }
    let Ok(key) = InodeKey::of(parent) else {
        return;
    };
    let mask = if is_directory {
        IN_CREATE | IN_ISDIR
    } else {
        IN_CREATE
    };
    queue_event(key, mask, Some(leaf));
}

pub fn close_note(parent: BorrowedFd<'_>, leaf: &CStr, file: BorrowedFd<'_>) -> Option<CloseNote> {
    let dentry = Arc::new(Mutex::new(Dentry {
        parent: InodeKey::of(parent).ok()?,
        leaf: leaf.to_owned(),
        file: InodeKey::of(file).ok()?,
    }));
    let mut open = OPEN_DENTRIES.lock().expect("fs_notify open dentries");
    open.retain(|weak| weak.strong_count() > 0);
    open.push(Arc::downgrade(&dentry));
    Some(CloseNote { dentry })
}

fn open_dentries() -> Vec<Arc<Mutex<Dentry>>> {
    OPEN_DENTRIES
        .lock()
        .expect("fs_notify open dentries")
        .iter()
        .filter_map(Weak::upgrade)
        .collect()
}

pub fn directory_key(parent: BorrowedFd<'_>, leaf: &CStr) -> Option<InodeKey> {
    if !watching() {
        return None;
    }
    let stat = rustix::fs::statat(parent, leaf, AtFlags::SYMLINK_NOFOLLOW).ok()?;
    (FileType::from_raw_mode(stat.st_mode) == FileType::Directory)
        .then(|| InodeKey::from_stat(stat))
}

pub fn removed_directory(key: InodeKey) {
    for instance in live_instances() {
        let mut state = instance.state();
        let removed = state
            .watches
            .iter()
            .filter(|(_, (watched, _))| *watched == key)
            .map(|(wd, _)| *wd)
            .collect::<Vec<_>>();
        for wd in removed {
            state.watches.remove(&wd);
            ACTIVE_WATCHES.fetch_sub(1, Ordering::SeqCst);
            state.queue.push_back(Event {
                wd,
                mask: IN_IGNORED,
                name: None,
            });
        }
    }
}

pub fn unsynthesized(parent: BorrowedFd<'_>, operation: &str) -> Option<io::Error> {
    if !watching() {
        return None;
    }
    let key = InodeKey::of(parent).ok()?;
    let watched = live_instances().iter().any(|instance| {
        instance
            .state()
            .watches
            .values()
            .any(|(watched, mask)| *watched == key && mask & WIRED_EVENTS != 0)
    });
    if !watched {
        return None;
    }
    let message = format!(
        "{operation} in a directory watched for inotify events would report host artifacts\nnext missing primitive: inotify events for {operation}"
    );
    crate::missing::record_from_error_message(&message);
    Some(io::Error::new(io::ErrorKind::Unsupported, message))
}

fn watching() -> bool {
    ACTIVE_WATCHES.load(Ordering::SeqCst) != 0
}

fn live_instances() -> Vec<Arc<Instance>> {
    INSTANCES
        .lock()
        .expect("fs_notify instances")
        .iter()
        .filter_map(Weak::upgrade)
        .collect()
}

fn queue_event(key: InodeKey, mask: u32, name: Option<&CStr>) {
    if !watching() {
        return;
    }
    for instance in live_instances() {
        let mut state = instance.state();
        let mut matching = state
            .watches
            .iter()
            .filter(|(_, (watched, watched_mask))| *watched == key && watched_mask & mask != 0)
            .map(|(wd, _)| *wd)
            .collect::<Vec<_>>();
        matching.sort_unstable();
        for wd in matching {
            state.queue.push_back(Event {
                wd,
                mask,
                name: name.map(CStr::to_owned),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::atomic::AtomicU64;

    use super::*;
    use crate::capability_fs::Directory;

    static FIXTURE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    struct Fixture {
        root: PathBuf,
    }

    impl Fixture {
        fn new(name: &str) -> Self {
            let sequence = FIXTURE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir().join(format!(
                "winedroid-fs-notify-{name}-{}-{sequence}",
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&root);
            std::fs::create_dir_all(root.join("watched")).unwrap();
            Self { root }
        }

        fn root(&self) -> Directory {
            Directory::open_ambient(&self.root).unwrap()
        }

        fn watch(&self, mask: u32) -> (Arc<Instance>, i32) {
            let instance = Instance::new();
            let watched = self.root().open_child(c"watched").unwrap();
            let wd = instance
                .add_watch(InodeKey::of(watched.as_fd()).unwrap(), mask)
                .unwrap();
            (instance, wd)
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    fn components(path: &str) -> Vec<CString> {
        path.split('/')
            .map(|part| CString::new(part).unwrap())
            .collect()
    }

    fn event(wd: i32, mask: u32, name: Option<&str>) -> Event {
        Event {
            wd,
            mask,
            name: name.map(|name| CString::new(name).unwrap()),
        }
    }

    #[test]
    fn inotify_h_create_then_close_write_for_a_new_file_written_and_closed() {
        let fixture = Fixture::new("create_close");
        let (instance, wd) = fixture.watch(WIRED_EVENTS);
        let file = fixture
            .root()
            .open_truncate(&components("watched/a"))
            .unwrap();
        assert_eq!(
            instance.take_events(),
            vec![event(wd, IN_CREATE, Some("a"))],
            "inotify(7) IN_CREATE is queued when open(O_CREAT) creates the entry"
        );
        drop(file);
        assert_eq!(
            instance.take_events(),
            vec![event(wd, IN_CLOSE_WRITE, Some("a"))],
            "inotify(7) IN_CLOSE_WRITE is queued when the writable descriptor closes"
        );
    }

    #[test]
    fn inotify_h_existing_truncate_gives_only_close_write_and_read_gives_nothing() {
        let fixture = Fixture::new("existing");
        std::fs::write(fixture.root.join("watched/old"), b"x").unwrap();
        let (instance, wd) = fixture.watch(WIRED_EVENTS);
        let root = fixture.root();
        drop(root.open_truncate(&components("watched/old")).unwrap());
        drop(root.open_read(&components("watched/old")).unwrap());
        assert_eq!(
            instance.take_events(),
            vec![event(wd, IN_CLOSE_WRITE, Some("old"))],
            "inotify(7): O_TRUNC of an existing file is no IN_CREATE, and a read only close is IN_CLOSE_NOWRITE"
        );
    }

    #[test]
    fn libcore_create_file_exclusively0_gives_create_then_close_write() {
        let fixture = Fixture::new("create_new");
        let (instance, wd) = fixture.watch(WIRED_EVENTS);
        drop(fixture.root().create_new(&components("watched/n")).unwrap());
        assert_eq!(
            instance.take_events(),
            vec![
                event(wd, IN_CREATE, Some("n")),
                event(wd, IN_CLOSE_WRITE, Some("n"))
            ],
            "UnixFileSystem_md.c createFileExclusively0 opens O_RDWR|O_CREAT|O_EXCL and closes"
        );
    }

    #[test]
    fn inotify_h_mkdir_all_reports_is_dir_only_in_the_watched_parent() {
        let fixture = Fixture::new("mkdir_all");
        let (instance, wd) = fixture.watch(WIRED_EVENTS);
        assert!(fixture
            .root()
            .mkdir_all(&components("watched/one/two"))
            .unwrap());
        assert_eq!(
            instance.take_events(),
            vec![event(wd, IN_CREATE | IN_ISDIR, Some("one"))],
            "inotify.h IN_ISDIR marks a created directory; the nested level is in an unwatched directory"
        );
        assert!(!fixture
            .root()
            .mkdir_all(&components("watched/one"))
            .unwrap());
        assert!(
            instance.take_events().is_empty(),
            "an existing level is not created again"
        );
        fixture.root().mkdir(&components("watched/single")).unwrap();
        assert_eq!(
            instance.take_events(),
            vec![event(wd, IN_CREATE | IN_ISDIR, Some("single"))],
            "inotify.h IN_ISDIR marks File.mkdir in the watched directory"
        );
    }

    #[test]
    fn inotify_h_mask_filters_events_and_same_inode_returns_same_wd() {
        let fixture = Fixture::new("mask");
        let (instance, wd) = fixture.watch(IN_CLOSE_WRITE);
        let watched = fixture.root().open_child(c"watched").unwrap();
        let key = InodeKey::of(watched.as_fd()).unwrap();
        drop(
            fixture
                .root()
                .open_truncate(&components("watched/f"))
                .unwrap(),
        );
        assert_eq!(
            instance.take_events(),
            vec![event(wd, IN_CLOSE_WRITE, Some("f"))],
            "inotify(7) only events in the watch mask are queued"
        );
        assert_eq!(
            instance.add_watch(key, IN_CREATE),
            Some(wd),
            "inotify_add_watch(2) on an inode already watched returns the same descriptor"
        );
        drop(
            fixture
                .root()
                .open_truncate(&components("watched/g"))
                .unwrap(),
        );
        assert_eq!(
            instance.take_events(),
            vec![event(wd, IN_CREATE, Some("g"))],
            "inotify_add_watch(2) without IN_MASK_ADD replaces the mask"
        );
        assert_eq!(
            instance.add_watch(key, 0),
            None,
            "inotify.h IN_ALL_EVENTS: a mask with no event bit is EINVAL"
        );
    }

    #[test]
    fn inotify_watches_the_inode_so_a_dot_dot_spelling_reaches_the_same_watch() {
        let fixture = Fixture::new("inode");
        let (instance, wd) = fixture.watch(WIRED_EVENTS);
        std::fs::create_dir(fixture.root.join("watched/sub")).unwrap();
        drop(
            fixture
                .root()
                .create_new(&components("watched/sub/../x"))
                .unwrap(),
        );
        assert_eq!(
            instance.take_events(),
            vec![
                event(wd, IN_CREATE, Some("x")),
                event(wd, IN_CLOSE_WRITE, Some("x"))
            ],
            "inotify(7) watches an inode, independent of the path spelling"
        );
    }

    #[test]
    fn inotify_close_write_keeps_the_name_of_a_file_unlinked_while_open() {
        let fixture = Fixture::new("unlinked");
        let (instance, wd) = fixture.watch(IN_CLOSE_WRITE);
        let root = fixture.root();
        let file = root.open_truncate(&components("watched/gone")).unwrap();
        root.remove(&components("watched/gone")).unwrap();
        drop(file);
        assert_eq!(
            instance.take_events(),
            vec![event(wd, IN_CLOSE_WRITE, Some("gone"))],
            "inotify(7) without IN_EXCL_UNLINK reports events of unlinked children"
        );
    }

    #[test]
    fn inotify_h_removing_the_watched_directory_queues_ignored_and_drops_the_watch() {
        let fixture = Fixture::new("removed");
        let (instance, wd) = fixture.watch(WIRED_EVENTS);
        let root = fixture.root();
        root.remove(&components("watched")).unwrap();
        assert_eq!(
            instance.take_events(),
            vec![event(wd, IN_IGNORED, None)],
            "inotify.h IN_IGNORED is queued when the watched directory is removed"
        );
        assert!(
            !instance.rm_watch(wd),
            "inotify(7) IN_IGNORED means the watch was removed, so inotify_rm_watch(2) is EINVAL"
        );
        root.mkdir(&components("watched")).unwrap();
        drop(root.create_new(&components("watched/late")).unwrap());
        assert!(
            instance.take_events().is_empty(),
            "a new directory with the old name is a different inode"
        );
    }

    #[test]
    fn inotify_h_rename_over_the_watched_directory_queues_ignored() {
        let fixture = Fixture::new("renamed_over");
        let (instance, wd) = fixture.watch(WIRED_EVENTS);
        let root = fixture.root();
        root.mkdir(&components("replacement")).unwrap();
        root.rename(&components("replacement"), &root, &components("watched"))
            .unwrap();
        assert_eq!(
            instance.take_events(),
            vec![event(wd, IN_IGNORED, None)],
            "inotify.h IN_IGNORED is queued when rename(2) replaces the watched directory"
        );
        assert!(
            !instance.rm_watch(wd),
            "inotify(7) IN_IGNORED means the watch was removed, so inotify_rm_watch(2) is EINVAL"
        );
    }

    #[test]
    fn rename_2_of_the_watched_directory_onto_itself_changes_nothing_and_keeps_the_watch() {
        let fixture = Fixture::new("renamed_onto_itself");
        let (instance, wd) = fixture.watch(WIRED_EVENTS);
        let root = fixture.root();
        root.rename(&components("watched"), &root, &components("watched"))
            .unwrap();
        root.rename(
            &components("watched"),
            &root,
            &components("watched/../watched"),
        )
        .unwrap();
        drop(root.create_new(&components("watched/kept")).unwrap());
        assert_eq!(
            instance.take_events(),
            vec![
                event(wd, IN_CREATE, Some("kept")),
                event(wd, IN_CLOSE_WRITE, Some("kept"))
            ],
            "rename(2): when both names are the same file, rename does nothing and succeeds"
        );
    }

    #[test]
    fn inotify_close_write_follows_a_rename_of_the_open_file() {
        let fixture = Fixture::new("renamed_open");
        std::fs::create_dir(fixture.root.join("elsewhere")).unwrap();
        let (instance, wd) = fixture.watch(WIRED_EVENTS);
        let root = fixture.root();

        let file = root
            .open_truncate(&components("watched/report.tmp"))
            .unwrap();
        root.rename(
            &components("watched/report.tmp"),
            &root,
            &components("watched/report.txt"),
        )
        .unwrap();
        drop(file);
        assert_eq!(
            instance.take_events(),
            vec![
                event(wd, IN_CREATE, Some("report.tmp")),
                event(wd, IN_CLOSE_WRITE, Some("report.txt"))
            ],
            "inotify(7) reports the close through the current name of the file"
        );

        let file = root.open_truncate(&components("elsewhere/in")).unwrap();
        root.rename(
            &components("elsewhere/in"),
            &root,
            &components("watched/in"),
        )
        .unwrap();
        let out = root.open_truncate(&components("watched/out")).unwrap();
        root.rename_noreplace(
            &components("watched/out"),
            &root,
            &components("elsewhere/out"),
        )
        .unwrap();
        instance.take_events();
        drop(file);
        drop(out);
        assert_eq!(
            instance.take_events(),
            vec![event(wd, IN_CLOSE_WRITE, Some("in"))],
            "inotify(7) reports the close to the directory that holds the file at close time"
        );
    }

    #[test]
    fn inotify_h_rm_watch_queues_ignored_and_unknown_wd_fails() {
        let fixture = Fixture::new("rm_watch");
        let (instance, wd) = fixture.watch(WIRED_EVENTS);
        assert!(instance.rm_watch(wd));
        assert!(
            !instance.rm_watch(wd),
            "inotify_rm_watch(2) of an unknown descriptor is EINVAL"
        );
        assert_eq!(
            instance.take_events(),
            vec![event(wd, IN_IGNORED, None)],
            "inotify.h IN_IGNORED follows inotify_rm_watch"
        );
        drop(
            fixture
                .root()
                .create_new(&components("watched/after"))
                .unwrap(),
        );
        assert!(instance.take_events().is_empty());
    }

    #[test]
    fn atomic_replace_in_a_watched_directory_is_a_missing_primitive() {
        let fixture = Fixture::new("atomic");
        let root = fixture.root();
        root.atomic_replace(&components("watched/store"), b"1")
            .unwrap();
        let (instance, _wd) = fixture.watch(WIRED_EVENTS);
        let error = root
            .atomic_replace(&components("watched/store"), b"2")
            .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::Unsupported);
        assert!(
            error
                .to_string()
                .contains("next missing primitive: inotify events for atomic_replace"),
            "{error}"
        );
        assert!(
            instance.take_events().is_empty(),
            "the temporary file and rename are never reported"
        );
        assert_eq!(
            std::fs::read(fixture.root.join("watched/store")).unwrap(),
            b"1"
        );
    }
}
