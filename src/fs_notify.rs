use std::collections::{BTreeMap, HashMap, VecDeque};
use std::ffi::{CStr, CString};
use std::fs::File;
use std::io;
use std::ops::{Deref, DerefMut};
use std::os::fd::{AsFd, BorrowedFd};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, Weak};

use rustix::fs::{AtFlags, FileType, Stat};

// cite: bionic inotify.h#IN_CLOSE_WRITE, IN_CREATE, IN_DELETE_SELF, IN_IGNORED, IN_ISDIR, IN_ALL_EVENTS
pub const IN_CLOSE_WRITE: u32 = 0x0000_0008;
pub const IN_CREATE: u32 = 0x0000_0100;
pub const IN_DELETE_SELF: u32 = 0x0000_0400;
pub const IN_IGNORED: u32 = 0x0000_8000;
pub const IN_ISDIR: u32 = 0x4000_0000;
const IN_ALL_EVENTS: u32 = 0x0000_0fff;
pub const WIRED_EVENTS: u32 = IN_CREATE | IN_CLOSE_WRITE | IN_DELETE_SELF;
pub const ANY_WATCH: u32 = u32::MAX;
const CHILD_EVENTS: u32 = IN_CREATE | IN_CLOSE_WRITE;

static INSTANCES: Mutex<Vec<Weak<Instance>>> = Mutex::new(Vec::new());
static ACTIVE_WATCHES: AtomicUsize = AtomicUsize::new(0);
static NAMES: Mutex<Names> = Mutex::new(Names {
    dentries: BTreeMap::new(),
    openings: BTreeMap::new(),
    deferred: BTreeMap::new(),
});
static REMOVING: Mutex<()> = Mutex::new(());
static NEXT_ID: AtomicU64 = AtomicU64::new(0);

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

    pub fn at(parent: BorrowedFd<'_>, leaf: &CStr) -> Option<Self> {
        rustix::fs::statat(parent, leaf, AtFlags::SYMLINK_NOFOLLOW)
            .ok()
            .map(Self::from_stat)
    }

    pub fn from_raw(device: u64, inode: u64) -> Self {
        Self { device, inode }
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

    fn state(&self) -> MutexGuard<'_, InstanceState> {
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

struct Names {
    dentries: BTreeMap<u64, Dentry>,
    openings: BTreeMap<u64, Pending>,
    deferred: BTreeMap<u64, Deferred>,
}

impl Names {
    fn is_empty(&self) -> bool {
        self.dentries.is_empty() && self.openings.is_empty()
    }

    fn referenced(&self, dentry: u64) -> bool {
        self.dentries.values().any(|other| other.dentry == dentry)
    }

    fn has_children(&self, directory: InodeKey) -> bool {
        self.dentries
            .values()
            .any(|other| other.parent == directory)
    }

    fn live_dentry(&self, parent: InodeKey, leaf: &CStr, file: InodeKey) -> Option<u64> {
        self.dentries
            .values()
            .find(|other| other.unlinked.is_none() && other.names(parent, leaf, file))
            .map(|other| other.dentry)
    }

    fn register(&mut self, parent: InodeKey, leaf: &CStr, stat: Stat) -> u64 {
        let file = InodeKey::from_stat(stat);
        let dentry = self.live_dentry(parent, leaf, file).unwrap_or_else(next_id);
        let id = next_id();
        self.dentries.insert(
            id,
            Dentry {
                dentry,
                parent,
                leaf: leaf.to_owned(),
                file,
                directory: FileType::from_raw_mode(stat.st_mode) == FileType::Directory,
                unlinked: None,
                orphan: false,
            },
        );
        id
    }

    // cite: bionic inotify.h#IN_DELETE_SELF, IN_IGNORED
    fn release(&mut self, entry: Dentry) -> Vec<(InodeKey, bool)> {
        let mut deleted = Vec::new();
        let mut current = entry;
        while current.unlinked.is_some() {
            if self.referenced(current.dentry) {
                break;
            }
            if current.directory && self.has_children(current.file) {
                self.dentries.insert(
                    next_id(),
                    Dentry {
                        orphan: true,
                        ..current
                    },
                );
                break;
            }
            if current.unlinked == Some(true) {
                deleted.push((current.file, current.directory));
            }
            if self.has_children(current.parent) {
                break;
            }
            let orphan = self
                .dentries
                .iter()
                .find(|(_, other)| other.orphan && other.file == current.parent)
                .map(|(id, _)| *id);
            let Some(orphan) = orphan else {
                break;
            };
            current = self.dentries.remove(&orphan).expect("orphan listed above");
        }
        deleted
    }

    fn resolve_opening(
        &mut self,
        opening: u64,
        stat: Option<Stat>,
    ) -> (Option<u64>, Vec<(InodeKey, bool)>) {
        let Some(pending) = self.openings.remove(&opening) else {
            return (None, Vec::new());
        };
        let hold = stat.map(|stat| self.register(pending.parent, &pending.leaf, stat));
        let file = stat.map(InodeKey::from_stat);
        let mut deleted = Vec::new();
        for id in pending.deferred {
            let Some(deferred) = self.deferred.get_mut(&id) else {
                continue;
            };
            if Some(deferred.file) == file {
                deferred.referenced = true;
                let entry = hold.and_then(|hold| self.dentries.get_mut(&hold));
                if let Some(entry) = entry {
                    entry.unlinked = Some(deferred.last_link);
                }
            }
            deferred.waiting -= 1;
            if deferred.waiting == 0 {
                let deferred = self.deferred.remove(&id).expect("deferred listed above");
                if deferred.last_link && !deferred.referenced {
                    deleted.push((deferred.file, deferred.directory));
                }
            }
        }
        (hold, deleted)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Dentry {
    dentry: u64,
    parent: InodeKey,
    leaf: CString,
    file: InodeKey,
    directory: bool,
    unlinked: Option<bool>,
    orphan: bool,
}

impl Dentry {
    fn names(&self, parent: InodeKey, leaf: &CStr, file: InodeKey) -> bool {
        self.parent == parent && self.leaf.as_c_str() == leaf && self.file == file
    }
}

struct Pending {
    parent: InodeKey,
    leaf: CString,
    deferred: Vec<u64>,
}

struct Deferred {
    file: InodeKey,
    directory: bool,
    last_link: bool,
    waiting: usize,
    referenced: bool,
}

#[derive(Debug)]
pub struct Opening {
    id: u64,
}

impl Opening {
    pub fn begin(parent: BorrowedFd<'_>, leaf: &CStr) -> Option<Self> {
        let parent = InodeKey::of(parent).ok()?;
        let id = next_id();
        names().openings.insert(
            id,
            Pending {
                parent,
                leaf: leaf.to_owned(),
                deferred: Vec::new(),
            },
        );
        Some(Self { id })
    }

    pub fn opened(self, file: BorrowedFd<'_>, close_write: bool) -> Option<Hold> {
        let stat = rustix::fs::fstat(file).ok();
        self.finish(stat, close_write)
    }

    fn finish(self, stat: Option<Stat>, close_write: bool) -> Option<Hold> {
        let (hold, deleted) = names().resolve_opening(self.id, stat);
        for (file, directory) in deleted {
            self::deleted(file, directory);
        }
        hold.map(|id| Hold { id, close_write })
    }
}

impl Drop for Opening {
    fn drop(&mut self) {
        let (_, deleted) = names().resolve_opening(self.id, None);
        for (file, directory) in deleted {
            self::deleted(file, directory);
        }
    }
}

#[derive(Debug)]
pub struct Hold {
    id: u64,
    close_write: bool,
}

impl Hold {
    pub fn open(opening: Option<Opening>, file: BorrowedFd<'_>, close_write: bool) -> Option<Self> {
        opening?.opened(file, close_write)
    }

    pub fn at(parent: BorrowedFd<'_>, leaf: &CStr) -> Option<Self> {
        let opening = Opening::begin(parent, leaf)?;
        let stat = rustix::fs::statat(parent, leaf, AtFlags::SYMLINK_NOFOLLOW).ok();
        opening.finish(stat, false)
    }

    pub fn for_host_path(path: &Path) -> Option<Self> {
        let parent = File::open(path.parent()?).ok()?;
        let leaf = CString::new(path.file_name()?.as_bytes()).ok()?;
        Self::at(parent.as_fd(), &leaf)
    }
}

impl Drop for Hold {
    fn drop(&mut self) {
        let (entry, deleted) = {
            let mut names = names();
            let Some(entry) = names.dentries.remove(&self.id) else {
                return;
            };
            let deleted = names.release(entry.clone());
            (entry, deleted)
        };
        if self.close_write {
            queue_event(entry.parent, IN_CLOSE_WRITE, Some(&entry.leaf));
            queue_event(entry.file, IN_CLOSE_WRITE, None);
        }
        for (file, directory) in deleted {
            self::deleted(file, directory);
        }
    }
}

pub struct Removal {
    parent: InodeKey,
    leaf: CString,
    file: InodeKey,
    directory: bool,
    last_link: bool,
    _serial: Option<MutexGuard<'static, ()>>,
}

impl Removal {
    pub fn prepare(parent: BorrowedFd<'_>, leaf: &CStr) -> Option<Self> {
        if !watching() && names().is_empty() {
            return None;
        }
        let serial = removing();
        let parent_key = InodeKey::of(parent).ok()?;
        let stat = rustix::fs::statat(parent, leaf, AtFlags::SYMLINK_NOFOLLOW).ok()?;
        Some(Self {
            _serial: Some(serial),
            ..Self::of(parent_key, leaf, stat)
        })
    }

    fn of(parent: InodeKey, leaf: &CStr, stat: Stat) -> Self {
        let directory = FileType::from_raw_mode(stat.st_mode) == FileType::Directory;
        Self {
            parent,
            leaf: leaf.to_owned(),
            file: InodeKey::from_stat(stat),
            directory,
            last_link: directory || stat.st_nlink <= 1,
            _serial: None,
        }
    }

    // cite: bionic inotify.h#IN_DELETE_SELF, IN_IGNORED
    pub fn finish(self) {
        let mut names = names();
        let mut referenced = false;
        for dentry in names.dentries.values_mut() {
            if dentry.file != self.file {
                continue;
            }
            if dentry.unlinked.is_some() {
                dentry.unlinked = Some(self.last_link);
            } else if dentry.names(self.parent, &self.leaf, self.file) {
                dentry.unlinked = Some(self.last_link);
                referenced = true;
            }
        }
        if !referenced && self.directory && names.has_children(self.file) {
            names.dentries.insert(
                next_id(),
                Dentry {
                    dentry: next_id(),
                    parent: self.parent,
                    leaf: self.leaf.clone(),
                    file: self.file,
                    directory: true,
                    unlinked: Some(true),
                    orphan: true,
                },
            );
            referenced = true;
        }
        let openings = names
            .openings
            .iter()
            .filter(|(_, pending)| {
                pending.parent == self.parent && pending.leaf.as_c_str() == self.leaf.as_c_str()
            })
            .map(|(id, _)| *id)
            .collect::<Vec<_>>();
        if !referenced && !openings.is_empty() {
            let id = next_id();
            names.deferred.insert(
                id,
                Deferred {
                    file: self.file,
                    directory: self.directory,
                    last_link: self.last_link,
                    waiting: openings.len(),
                    referenced: false,
                },
            );
            for opening in openings {
                names
                    .openings
                    .get_mut(&opening)
                    .expect("opening listed above")
                    .deferred
                    .push(id);
            }
            return;
        }
        drop(names);
        if self.last_link && !referenced {
            deleted(self.file, self.directory);
        }
    }
}

#[derive(Default)]
pub struct Rename {
    moves: Vec<(InodeKey, CString, InodeKey, InodeKey, CString)>,
    replaced: Option<Removal>,
    _serial: Option<MutexGuard<'static, ()>>,
}

impl Rename {
    pub fn prepare(
        from_parent: BorrowedFd<'_>,
        from_leaf: &CStr,
        to_parent: BorrowedFd<'_>,
        to_leaf: &CStr,
        exchange: bool,
    ) -> Self {
        if !watching() && names().is_empty() {
            return Self::default();
        }
        let serial = removing();
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
            from_key,
            from_leaf.to_owned(),
            source,
            to_key,
            to_leaf.to_owned(),
        )];
        if let (true, Some(target)) = (exchange, target_key) {
            moves.push((
                to_key,
                to_leaf.to_owned(),
                target,
                from_key,
                from_leaf.to_owned(),
            ));
        }
        let replaced = target
            .filter(|_| !exchange)
            .map(|stat| Removal::of(to_key, to_leaf, stat));
        Self {
            moves,
            replaced,
            _serial: Some(serial),
        }
    }

    pub fn finish(self) {
        let Self {
            moves,
            replaced,
            _serial,
        } = self;
        for dentry in names().dentries.values_mut() {
            let moved = moves.iter().find(|(parent, leaf, file, _, _)| {
                dentry.unlinked.is_none() && dentry.names(*parent, leaf, *file)
            });
            if let Some((_, _, _, parent, leaf)) = moved {
                dentry.parent = *parent;
                dentry.leaf = leaf.clone();
            }
        }
        if let Some(replaced) = replaced {
            replaced.finish();
        }
    }
}

#[derive(Debug)]
pub struct NotifiedFile {
    file: File,
    hold: Option<Hold>,
}

impl NotifiedFile {
    pub fn new(file: File, hold: Option<Hold>) -> Self {
        Self { file, hold }
    }

    pub fn into_hold(self) -> Option<Hold> {
        let Self { file, hold } = self;
        drop(file);
        hold
    }

    pub fn into_parts(self) -> (File, Option<Hold>) {
        (self.file, self.hold)
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

pub fn unsynthesized(
    parent: Option<BorrowedFd<'_>>,
    target: Option<(InodeKey, u32)>,
    operation: &str,
) -> Option<io::Error> {
    if !watching() {
        return None;
    }
    let parent = parent.and_then(|parent| InodeKey::of(parent).ok());
    let watched = live_instances().iter().any(|instance| {
        instance.state().watches.values().any(|(watched, mask)| {
            (Some(*watched) == parent && mask & CHILD_EVENTS != 0)
                || target.is_some_and(|(key, events)| *watched == key && mask & events != 0)
        })
    });
    if !watched {
        return None;
    }
    let message = format!(
        "{operation} on a file or directory watched for inotify events would report host artifacts\nnext missing primitive: inotify events for {operation}"
    );
    crate::missing::record_from_error_message(&message);
    Some(io::Error::new(io::ErrorKind::Unsupported, message))
}

fn names() -> MutexGuard<'static, Names> {
    NAMES.lock().expect("fs_notify names")
}

fn removing() -> MutexGuard<'static, ()> {
    REMOVING.lock().expect("fs_notify removals")
}

fn next_id() -> u64 {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
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

fn watches_of(state: &InstanceState, key: InodeKey) -> Vec<i32> {
    let mut matching = state
        .watches
        .iter()
        .filter(|(_, (watched, _))| *watched == key)
        .map(|(wd, _)| *wd)
        .collect::<Vec<_>>();
    matching.sort_unstable();
    matching
}

fn queue_event(key: InodeKey, mask: u32, name: Option<&CStr>) {
    if !watching() {
        return;
    }
    for instance in live_instances() {
        let mut state = instance.state();
        for wd in watches_of(&state, key) {
            if state.watches[&wd].1 & mask == 0 {
                continue;
            }
            state.queue.push_back(Event {
                wd,
                mask,
                name: name.map(CStr::to_owned),
            });
        }
    }
}

// cite: bionic inotify.h#IN_DELETE_SELF, IN_IGNORED, IN_ISDIR
fn deleted(key: InodeKey, directory: bool) {
    if !watching() {
        return;
    }
    let delete_self = if directory {
        IN_DELETE_SELF | IN_ISDIR
    } else {
        IN_DELETE_SELF
    };
    for instance in live_instances() {
        let mut state = instance.state();
        for wd in watches_of(&state, key) {
            let (_, mask) = state.watches.remove(&wd).expect("watch listed above");
            ACTIVE_WATCHES.fetch_sub(1, Ordering::SeqCst);
            if mask & IN_DELETE_SELF != 0 {
                state.queue.push_back(Event {
                    wd,
                    mask: delete_self,
                    name: None,
                });
            }
            state.queue.push_back(Event {
                wd,
                mask: IN_IGNORED,
                name: None,
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

        fn watch_file(&self, instance: &Instance, relative: &str, mask: u32) -> i32 {
            let key = self.root().inode_key(&components(relative)).unwrap();
            instance.add_watch(key, mask).unwrap()
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
        let (instance, wd) = fixture.watch(IN_CREATE | IN_CLOSE_WRITE);
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
        let (instance, wd) = fixture.watch(IN_CREATE | IN_CLOSE_WRITE);
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
        let (instance, wd) = fixture.watch(IN_CREATE | IN_CLOSE_WRITE);
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
        let (instance, wd) = fixture.watch(IN_CREATE | IN_CLOSE_WRITE);
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
        let (instance, wd) = fixture.watch(IN_CREATE | IN_CLOSE_WRITE);
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
        let (instance, wd) = fixture.watch(IN_CREATE | IN_CLOSE_WRITE);
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
        let (instance, wd) = fixture.watch(IN_CREATE | IN_CLOSE_WRITE);
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
        let (instance, wd) = fixture.watch(IN_CREATE | IN_CLOSE_WRITE);
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
        let (instance, wd) = fixture.watch(IN_CREATE | IN_CLOSE_WRITE);
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
        let (instance, wd) = fixture.watch(IN_CREATE | IN_CLOSE_WRITE);
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
        let (instance, _wd) = fixture.watch(IN_CREATE | IN_CLOSE_WRITE);
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

    const FILE_MASK: u32 = IN_CLOSE_WRITE | IN_DELETE_SELF;

    #[test]
    fn inotify_7_file_watch_gets_close_write_without_a_name_after_the_parent_event() {
        let fixture = Fixture::new("file_close");
        std::fs::write(fixture.root.join("watched/f"), b"").unwrap();
        let (instance, directory_wd) = fixture.watch(IN_CLOSE_WRITE);
        let file_wd = fixture.watch_file(&instance, "watched/f", FILE_MASK);
        drop(
            fixture
                .root()
                .open_append(&components("watched/f"))
                .unwrap(),
        );
        drop(fixture.root().open_read(&components("watched/f")).unwrap());
        assert_eq!(
            instance.take_events(),
            vec![
                event(directory_wd, IN_CLOSE_WRITE, Some("f")),
                event(file_wd, IN_CLOSE_WRITE, None)
            ],
            "inotify(7): an event on a watched file inside a watched directory goes to both, the file watch with no name; a read only close is IN_CLOSE_NOWRITE"
        );
    }

    #[test]
    fn inotify_7_removing_the_last_link_of_a_watched_file_gives_delete_self_then_ignored() {
        let fixture = Fixture::new("file_delete");
        std::fs::write(fixture.root.join("watched/f"), b"").unwrap();
        let instance = Instance::new();
        let wd = fixture.watch_file(&instance, "watched/f", FILE_MASK);
        fixture.root().remove(&components("watched/f")).unwrap();
        assert_eq!(
            instance.take_events(),
            vec![event(wd, IN_DELETE_SELF, None), event(wd, IN_IGNORED, None)],
            "inotify.h IN_DELETE_SELF then IN_IGNORED when the watched file is deleted"
        );
        assert!(
            !instance.rm_watch(wd),
            "inotify(7) IN_IGNORED means the watch was removed"
        );
    }

    #[test]
    fn inotify_7_delete_self_without_the_bit_in_the_mask_still_queues_ignored() {
        let fixture = Fixture::new("file_delete_unmasked");
        std::fs::write(fixture.root.join("watched/f"), b"").unwrap();
        let instance = Instance::new();
        let wd = fixture.watch_file(&instance, "watched/f", IN_CLOSE_WRITE);
        fixture.root().remove(&components("watched/f")).unwrap();
        assert_eq!(
            instance.take_events(),
            vec![event(wd, IN_IGNORED, None)],
            "inotify(7) IN_IGNORED is queued for a deleted file whatever the mask"
        );
    }

    #[test]
    fn unlink_2_delete_self_of_an_open_file_waits_for_the_last_reference() {
        let fixture = Fixture::new("file_delete_open");
        std::fs::write(fixture.root.join("watched/f"), b"").unwrap();
        let instance = Instance::new();
        let wd = fixture.watch_file(&instance, "watched/f", FILE_MASK);
        let root = fixture.root();
        let writer = root.open_append(&components("watched/f")).unwrap();
        let reader = root.open_read(&components("watched/f")).unwrap();
        root.remove(&components("watched/f")).unwrap();
        assert!(
            instance.take_events().is_empty(),
            "unlink(2): if any process still has the file open the file remains in existence"
        );
        drop(writer);
        assert_eq!(
            instance.take_events(),
            vec![event(wd, IN_CLOSE_WRITE, None)],
            "inotify(7) IN_CLOSE_WRITE on the writable close while a reader still holds the inode"
        );
        drop(reader);
        assert_eq!(
            instance.take_events(),
            vec![event(wd, IN_DELETE_SELF, None), event(wd, IN_IGNORED, None)],
            "IN_DELETE_SELF and IN_IGNORED follow the release of the last reference, a read only one included"
        );
    }

    #[test]
    fn unlink_2_removing_one_of_two_hard_links_gives_no_delete_self() {
        let fixture = Fixture::new("hard_link");
        std::fs::write(fixture.root.join("watched/f"), b"").unwrap();
        std::fs::hard_link(
            fixture.root.join("watched/f"),
            fixture.root.join("watched/g"),
        )
        .unwrap();
        let instance = Instance::new();
        let wd = fixture.watch_file(&instance, "watched/f", FILE_MASK);
        let root = fixture.root();
        let reader = root.open_read(&components("watched/f")).unwrap();
        root.remove(&components("watched/f")).unwrap();
        drop(reader);
        assert!(
            instance.take_events().is_empty(),
            "unlink(2) deletes the file only when the name was its last link, open or not; IN_ATTRIB is not wired"
        );
        root.remove(&components("watched/g")).unwrap();
        assert_eq!(
            instance.take_events(),
            vec![event(wd, IN_DELETE_SELF, None), event(wd, IN_IGNORED, None)],
            "inotify.h IN_DELETE_SELF when the last link goes"
        );
    }

    #[test]
    fn dcache_d_delete_fires_for_an_unreferenced_last_link_while_another_unlinked_name_is_open() {
        let fixture = Fixture::new("hard_link_open");
        std::fs::write(fixture.root.join("watched/f"), b"").unwrap();
        std::fs::hard_link(
            fixture.root.join("watched/f"),
            fixture.root.join("watched/g"),
        )
        .unwrap();
        let instance = Instance::new();
        let wd = fixture.watch_file(&instance, "watched/f", FILE_MASK);
        let root = fixture.root();
        let reader = root.open_read(&components("watched/f")).unwrap();
        root.remove(&components("watched/f")).unwrap();
        root.remove(&components("watched/g")).unwrap();
        assert_eq!(
            instance.take_events(),
            vec![event(wd, IN_DELETE_SELF, None), event(wd, IN_IGNORED, None)],
            "Linux d_delete releases an unreferenced name at once and the inode has no links left"
        );
        drop(reader);
        assert!(instance.take_events().is_empty());
    }

    #[test]
    fn dcache_releasing_an_unlinked_name_fires_once_the_inode_has_no_links_even_if_another_is_open()
    {
        let fixture = Fixture::new("hard_link_both_open");
        std::fs::write(fixture.root.join("watched/f"), b"").unwrap();
        std::fs::hard_link(
            fixture.root.join("watched/f"),
            fixture.root.join("watched/g"),
        )
        .unwrap();
        let instance = Instance::new();
        let wd = fixture.watch_file(&instance, "watched/f", FILE_MASK);
        let root = fixture.root();
        let through_f = root.open_read(&components("watched/f")).unwrap();
        let through_g = root.open_read(&components("watched/g")).unwrap();
        root.remove(&components("watched/f")).unwrap();
        root.remove(&components("watched/g")).unwrap();
        assert!(instance.take_events().is_empty());
        drop(through_f);
        assert_eq!(
            instance.take_events(),
            vec![event(wd, IN_DELETE_SELF, None), event(wd, IN_IGNORED, None)],
            "Linux dentry_unlink_inode checks i_nlink when each name is released, so the first released name of a linkless inode fires"
        );
        drop(through_g);
        assert!(instance.take_events().is_empty());
    }

    #[test]
    fn dcache_an_unlinked_name_does_not_follow_a_rename_of_a_new_link_with_that_name() {
        let fixture = Fixture::new("relinked");
        std::fs::write(fixture.root.join("watched/f"), b"").unwrap();
        let instance = Instance::new();
        let wd = fixture.watch_file(&instance, "watched/f", FILE_MASK);
        let root = fixture.root();
        let old_name = root.open_read(&components("watched/f")).unwrap();
        std::fs::hard_link(
            fixture.root.join("watched/f"),
            fixture.root.join("watched/keep"),
        )
        .unwrap();
        root.remove(&components("watched/f")).unwrap();
        std::fs::hard_link(
            fixture.root.join("watched/keep"),
            fixture.root.join("watched/f"),
        )
        .unwrap();
        root.remove(&components("watched/keep")).unwrap();
        let new_name = root.open_read(&components("watched/f")).unwrap();
        root.rename(&components("watched/f"), &root, &components("watched/c"))
            .unwrap();
        root.remove(&components("watched/c")).unwrap();
        drop(new_name);
        assert_eq!(
            instance.take_events(),
            vec![event(wd, IN_DELETE_SELF, None), event(wd, IN_IGNORED, None)],
            "rename(2) moves the live name only; the unlinked old name does not pin the renamed one"
        );
        drop(old_name);
        assert!(instance.take_events().is_empty());
    }

    #[test]
    fn unlink_2_a_hold_taken_by_host_path_keeps_the_deleted_file_until_dropped() {
        let fixture = Fixture::new("host_path");
        std::fs::write(fixture.root.join("watched/lib.so"), b"elf").unwrap();
        let instance = Instance::new();
        let wd = fixture.watch_file(&instance, "watched/lib.so", IN_DELETE_SELF);
        let hold = Hold::for_host_path(&fixture.root.join("watched/lib.so")).unwrap();
        fixture
            .root()
            .remove(&components("watched/lib.so"))
            .unwrap();
        assert!(
            instance.take_events().is_empty(),
            "a mapped library references its file like an open descriptor"
        );
        drop(hold);
        assert_eq!(
            instance.take_events(),
            vec![event(wd, IN_DELETE_SELF, None), event(wd, IN_IGNORED, None)]
        );
    }

    #[test]
    fn rename_2_over_the_watched_file_gives_delete_self_then_ignored_after_its_last_close() {
        let fixture = Fixture::new("file_renamed_over");
        std::fs::write(fixture.root.join("watched/f"), b"old").unwrap();
        std::fs::write(fixture.root.join("watched/new"), b"new").unwrap();
        let instance = Instance::new();
        let wd = fixture.watch_file(&instance, "watched/f", FILE_MASK);
        let root = fixture.root();
        let reader = root.open_read(&components("watched/f")).unwrap();
        root.rename(&components("watched/new"), &root, &components("watched/f"))
            .unwrap();
        assert!(
            instance.take_events().is_empty(),
            "rename(2): the replaced file stays in existence while it is open"
        );
        drop(reader);
        assert_eq!(
            instance.take_events(),
            vec![event(wd, IN_DELETE_SELF, None), event(wd, IN_IGNORED, None)],
            "inotify.h IN_DELETE_SELF for the inode rename(2) replaced"
        );

        std::fs::write(fixture.root.join("watched/newer"), b"newer").unwrap();
        let wd = fixture.watch_file(&instance, "watched/f", FILE_MASK);
        root.rename(
            &components("watched/newer"),
            &root,
            &components("watched/f"),
        )
        .unwrap();
        assert_eq!(
            instance.take_events(),
            vec![event(wd, IN_DELETE_SELF, None), event(wd, IN_IGNORED, None)],
            "inotify.h IN_DELETE_SELF at once when nothing holds the replaced file"
        );
        assert_eq!(
            std::fs::read(fixture.root.join("watched/f")).unwrap(),
            b"newer"
        );
    }

    #[test]
    fn inotify_7_renaming_the_watched_file_away_keeps_the_watch() {
        let fixture = Fixture::new("file_renamed_away");
        std::fs::write(fixture.root.join("watched/f"), b"").unwrap();
        let instance = Instance::new();
        let wd = fixture.watch_file(&instance, "watched/f", FILE_MASK);
        let root = fixture.root();
        root.rename(
            &components("watched/f"),
            &root,
            &components("watched/moved"),
        )
        .unwrap();
        assert!(
            instance.take_events().is_empty(),
            "inotify(7) IN_MOVE_SELF is not in the mask and the inode lives on"
        );
        root.remove(&components("watched/moved")).unwrap();
        assert_eq!(
            instance.take_events(),
            vec![event(wd, IN_DELETE_SELF, None), event(wd, IN_IGNORED, None)],
            "inotify(7) watches the inode under its new name"
        );
    }

    #[test]
    fn inotify_h_directory_watch_with_delete_self_gets_it_with_is_dir_before_ignored() {
        let fixture = Fixture::new("dir_delete_self");
        let (instance, wd) = fixture.watch(IN_CREATE | IN_DELETE_SELF);
        fixture.root().remove(&components("watched")).unwrap();
        assert_eq!(
            instance.take_events(),
            vec![
                event(wd, IN_DELETE_SELF | IN_ISDIR, None),
                event(wd, IN_IGNORED, None)
            ],
            "inotify.h IN_DELETE_SELF, with IN_ISDIR for a directory, precedes IN_IGNORED"
        );
    }

    #[test]
    fn atomic_replace_of_a_watched_file_and_unlink_leaf_of_one_are_missing_primitives() {
        let fixture = Fixture::new("atomic_file");
        let root = fixture.root();
        root.atomic_replace(&components("watched/store"), b"1")
            .unwrap();
        let instance = Instance::new();
        let _wd = fixture.watch_file(&instance, "watched/store", IN_DELETE_SELF);
        let error = root
            .atomic_replace(&components("watched/store"), b"2")
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("next missing primitive: inotify events for atomic_replace"),
            "{error}"
        );
        let watched = root.open_child(c"watched").unwrap();
        let error = watched.unlink_leaf(c"store").unwrap_err();
        assert!(
            error
                .to_string()
                .contains("next missing primitive: inotify events for unlink_leaf"),
            "{error}"
        );
        assert!(instance.take_events().is_empty());
        assert_eq!(
            std::fs::read(fixture.root.join("watched/store")).unwrap(),
            b"1"
        );
    }

    #[test]
    fn dcache_a_removed_directory_lives_until_its_open_unlinked_child_is_released() {
        let fixture = Fixture::new("dir_child_held");
        std::fs::create_dir(fixture.root.join("watched/d")).unwrap();
        std::fs::write(fixture.root.join("watched/d/f"), b"").unwrap();
        let instance = Instance::new();
        let directory_wd = fixture.watch_file(&instance, "watched/d", IN_DELETE_SELF);
        let file_wd = fixture.watch_file(&instance, "watched/d/f", IN_DELETE_SELF);
        let root = fixture.root();
        let reader = root.open_read(&components("watched/d/f")).unwrap();
        root.remove(&components("watched/d/f")).unwrap();
        root.remove(&components("watched/d")).unwrap();
        assert!(
            instance.take_events().is_empty(),
            "Linux d_alloc takes a reference on the parent dentry, so vfs_rmdir leaves the directory to its last child"
        );
        drop(reader);
        assert_eq!(
            instance.take_events(),
            vec![
                event(file_wd, IN_DELETE_SELF, None),
                event(file_wd, IN_IGNORED, None),
                event(directory_wd, IN_DELETE_SELF | IN_ISDIR, None),
                event(directory_wd, IN_IGNORED, None)
            ],
            "Linux __dentry_kill releases the child, then the parent reference it held"
        );
    }

    #[test]
    fn dcache_a_held_removed_subdirectory_keeps_its_removed_parent_and_a_closed_parent_fd_waits() {
        let fixture = Fixture::new("dir_grandchild_held");
        std::fs::create_dir_all(fixture.root.join("watched/d/sub")).unwrap();
        let instance = Instance::new();
        let directory_wd = fixture.watch_file(&instance, "watched/d", IN_DELETE_SELF);
        let root = fixture.root();
        let d = root
            .open_child(c"watched")
            .unwrap()
            .open_child(c"d")
            .unwrap();
        let cwd = Hold::at(d.as_fd(), c"sub").unwrap();
        let directory_fd = Hold::at(root.open_child(c"watched").unwrap().as_fd(), c"d").unwrap();
        root.remove(&components("watched/d/sub")).unwrap();
        root.remove(&components("watched/d")).unwrap();
        drop(directory_fd);
        assert!(
            instance.take_events().is_empty(),
            "Linux dput of the directory descriptor leaves a dentry its removed subdirectory still references"
        );
        drop(cwd);
        assert_eq!(
            instance.take_events(),
            vec![
                event(directory_wd, IN_DELETE_SELF | IN_ISDIR, None),
                event(directory_wd, IN_IGNORED, None)
            ],
            "Linux __dentry_kill of the subdirectory drops the last reference on the removed directory"
        );
    }

    #[test]
    fn dcache_an_old_unlinked_dentry_is_released_apart_from_a_new_link_with_the_same_name() {
        let fixture = Fixture::new("same_name_dentries");
        std::fs::write(fixture.root.join("watched/a"), b"").unwrap();
        std::fs::hard_link(
            fixture.root.join("watched/a"),
            fixture.root.join("watched/b"),
        )
        .unwrap();
        let instance = Instance::new();
        let wd = fixture.watch_file(&instance, "watched/a", FILE_MASK);
        let root = fixture.root();
        let old_name = root.open_read(&components("watched/a")).unwrap();
        root.remove(&components("watched/a")).unwrap();
        std::fs::hard_link(
            fixture.root.join("watched/b"),
            fixture.root.join("watched/a"),
        )
        .unwrap();
        let new_name = root.open_read(&components("watched/a")).unwrap();
        root.remove(&components("watched/b")).unwrap();
        root.remove(&components("watched/a")).unwrap();
        assert!(instance.take_events().is_empty());
        drop(old_name);
        assert_eq!(
            instance.take_events(),
            vec![event(wd, IN_DELETE_SELF, None), event(wd, IN_IGNORED, None)],
            "Linux dentry_unlink_inode runs per dentry: the old a dentry is released while the new a dentry is open"
        );
        drop(new_name);
        assert!(instance.take_events().is_empty());
    }

    #[test]
    fn unlink_2_of_two_links_from_two_threads_deletes_the_inode_once() {
        let fixture = Fixture::new("concurrent_unlinks");
        std::fs::write(fixture.root.join("watched/a"), b"").unwrap();
        std::fs::hard_link(
            fixture.root.join("watched/a"),
            fixture.root.join("watched/b"),
        )
        .unwrap();
        let instance = Instance::new();
        let wd = fixture.watch_file(&instance, "watched/a", FILE_MASK);
        let watched = fixture.root().open_child(c"watched").unwrap();
        let first = Removal::prepare(watched.as_fd(), c"a").unwrap();
        let root = fixture.root.clone();
        let second = std::thread::spawn(move || {
            Directory::open_ambient(&root)
                .unwrap()
                .remove(&components("watched/b"))
                .unwrap();
        });
        std::thread::sleep(std::time::Duration::from_millis(100));
        rustix::fs::unlinkat(watched.as_fd(), c"a", AtFlags::empty()).unwrap();
        first.finish();
        second.join().unwrap();
        assert_eq!(
            instance.take_events(),
            vec![event(wd, IN_DELETE_SELF, None), event(wd, IN_IGNORED, None)],
            "Linux vfs_unlink drops i_nlink under the inode lock, so exactly one of two racing unlinks removes the last link"
        );
    }

    #[test]
    fn open_2_racing_unlink_2_holds_the_file_it_opened_and_a_failed_open_releases_the_removal() {
        let fixture = Fixture::new("open_races_unlink");
        std::fs::write(fixture.root.join("watched/f"), b"").unwrap();
        let instance = Instance::new();
        let wd = fixture.watch_file(&instance, "watched/f", FILE_MASK);
        let root = fixture.root();
        let watched = root.open_child(c"watched").unwrap();
        let opening = Opening::begin(watched.as_fd(), c"f").unwrap();
        let fd = rustix::fs::openat(
            watched.as_fd(),
            c"f",
            rustix::fs::OFlags::RDONLY,
            rustix::fs::Mode::empty(),
        )
        .unwrap();
        root.remove(&components("watched/f")).unwrap();
        let hold = opening.opened(fd.as_fd(), false);
        assert!(
            instance.take_events().is_empty(),
            "open(2) took its dentry reference before unlink(2), so the file lives while it is open"
        );
        drop(hold);
        assert_eq!(
            instance.take_events(),
            vec![event(wd, IN_DELETE_SELF, None), event(wd, IN_IGNORED, None)]
        );

        std::fs::write(fixture.root.join("watched/g"), b"").unwrap();
        let wd = fixture.watch_file(&instance, "watched/g", FILE_MASK);
        let opening = Opening::begin(watched.as_fd(), c"g").unwrap();
        root.remove(&components("watched/g")).unwrap();
        assert!(instance.take_events().is_empty());
        drop(opening);
        assert_eq!(
            instance.take_events(),
            vec![event(wd, IN_DELETE_SELF, None), event(wd, IN_IGNORED, None)],
            "an open(2) that found no file never referenced it"
        );
    }
}
