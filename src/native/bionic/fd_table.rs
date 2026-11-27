use std::any::Any;
use std::collections::HashMap;
use std::sync::atomic::{AtomicI32, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FdKind {
    Host,
    Dir,
    PathOnly,
    Pipe,
    Socket,
    EventFd,
    TimerFd,
    Epoll,
}

pub struct DirItem {
    pub inode: u64,
    pub name: Vec<u8>,
    pub kind: u8,
}

#[derive(Default)]
pub struct DirState {
    pub items: Option<Vec<DirItem>>,
    pub position: usize,
}

#[derive(Clone, Copy, Default)]
pub struct SyntheticStat {
    pub mode: u32,
    pub dev: (u32, u32),
    pub rdev: (u32, u32),
    pub zero_size: bool,
}

pub struct FdEntry {
    pub kind: FdKind,
    pub guest_path: String,
    pub linux_flags: AtomicI32,
    pub owner_tag: AtomicU64,
    pub seals: AtomicU32,
    pub synthetic: Option<SyntheticStat>,
    pub dir: Mutex<DirState>,
    pub payload: Option<Arc<dyn Any + Send + Sync>>,
    pub hold: Option<Arc<crate::fs_notify::Hold>>,
}

pub const SEALING_ALLOWED: u32 = 1 << 31;

impl FdEntry {
    pub fn new(kind: FdKind, guest_path: impl Into<String>, linux_flags: i32) -> Self {
        Self {
            kind,
            guest_path: guest_path.into(),
            linux_flags: AtomicI32::new(linux_flags),
            owner_tag: AtomicU64::new(0),
            seals: AtomicU32::new(0),
            synthetic: None,
            dir: Mutex::new(DirState::default()),
            payload: None,
            hold: None,
        }
    }

    pub fn with_synthetic(mut self, stat: SyntheticStat) -> Self {
        self.synthetic = Some(stat);
        self
    }

    pub fn with_payload(mut self, payload: Arc<dyn Any + Send + Sync>) -> Self {
        self.payload = Some(payload);
        self
    }

    pub fn flags(&self) -> i32 {
        self.linux_flags.load(Ordering::SeqCst)
    }

    pub fn payload_as<T: Any + Send + Sync>(&self) -> Option<Arc<T>> {
        self.payload.clone()?.downcast::<T>().ok()
    }

    pub fn duplicate(&self) -> FdEntry {
        let mut copy = FdEntry::new(self.kind, self.guest_path.clone(), self.flags());
        copy.seals = AtomicU32::new(self.seals.load(Ordering::SeqCst));
        copy.synthetic = self.synthetic;
        copy.payload = self.payload.clone();
        copy.hold = self.hold.clone();
        copy
    }
}

static TABLE: OnceLock<Mutex<HashMap<i32, Arc<FdEntry>>>> = OnceLock::new();

fn table() -> &'static Mutex<HashMap<i32, Arc<FdEntry>>> {
    TABLE.get_or_init(|| {
        let mut map = HashMap::new();
        for (fd, flags) in [(0, 0), (1, 1), (2, 1)] {
            map.insert(fd, Arc::new(FdEntry::new(FdKind::Host, "/dev/null", flags)));
        }
        Mutex::new(map)
    })
}

pub fn adopt(fd: i32, entry: FdEntry) -> Arc<FdEntry> {
    let entry = Arc::new(entry);
    table().lock().expect("fd table").insert(fd, entry.clone());
    entry
}

pub fn get(fd: i32) -> Option<Arc<FdEntry>> {
    table().lock().expect("fd table").get(&fd).cloned()
}

pub fn take(fd: i32) -> Option<Arc<FdEntry>> {
    table().lock().expect("fd table").remove(&fd)
}

pub fn list() -> Vec<i32> {
    let mut fds: Vec<i32> = table().lock().expect("fd table").keys().copied().collect();
    fds.sort_unstable();
    fds
}

pub fn contains(fd: i32) -> bool {
    table().lock().expect("fd table").contains_key(&fd)
}

pub fn adopt_duplicate(source: &FdEntry, new_fd: i32, cloexec: bool) -> Arc<FdEntry> {
    let copy = source.duplicate();
    let _ = cloexec;
    adopt(new_fd, copy)
}
