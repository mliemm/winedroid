use std::collections::BTreeMap;
use std::sync::Mutex;

#[derive(Clone)]
pub struct FileBacking {
    pub guest_path: String,
    pub offset: u64,
    pub device: (u32, u32),
    pub inode: u64,
    pub close_note: Option<std::sync::Arc<crate::fs_notify::CloseNote>>,
}

#[derive(Clone)]
pub struct MapRecord {
    pub len: usize,
    pub prot: i32,
    pub shared: bool,
    pub no_reserve: bool,
    pub file: Option<FileBacking>,
    pub name: Option<String>,
}

static REGISTRY: Mutex<BTreeMap<usize, MapRecord>> = Mutex::new(BTreeMap::new());

fn trim_front(record: &MapRecord, advance: usize) -> MapRecord {
    let mut trimmed = record.clone();
    trimmed.len -= advance;
    if let Some(file) = trimmed.file.as_mut() {
        file.offset += advance as u64;
    }
    trimmed
}

fn carve(
    map: &mut BTreeMap<usize, MapRecord>,
    start: usize,
    end: usize,
) -> Vec<(usize, MapRecord)> {
    let overlapping: Vec<usize> = map
        .range(..end)
        .filter(|(base, record)| **base + record.len > start)
        .map(|(base, _)| *base)
        .collect();
    let mut removed = Vec::new();
    for base in overlapping {
        let record = map.remove(&base).expect("listed record");
        let record_end = base + record.len;
        if base < start {
            let mut head = record.clone();
            head.len = start - base;
            map.insert(base, head);
        }
        if record_end > end {
            map.insert(end, trim_front(&record, end - base));
        }
        let cut_start = base.max(start);
        let cut_end = record_end.min(end);
        let mut piece = trim_front(&record, cut_start - base);
        piece.len = cut_end - cut_start;
        removed.push((cut_start, piece));
    }
    removed
}

pub fn insert(start: usize, record: MapRecord) {
    let mut map = REGISTRY.lock().expect("map registry");
    carve(&mut map, start, start + record.len);
    map.insert(start, record);
}

pub fn remove_range(start: usize, len: usize) {
    carve(
        &mut REGISTRY.lock().expect("map registry"),
        start,
        start + len,
    );
}

pub fn update_range(start: usize, len: usize, change: impl Fn(&mut MapRecord)) {
    let mut map = REGISTRY.lock().expect("map registry");
    for (base, mut piece) in carve(&mut map, start, start + len) {
        change(&mut piece);
        map.insert(base, piece);
    }
}

pub fn snapshot() -> Vec<(usize, MapRecord)> {
    REGISTRY
        .lock()
        .expect("map registry")
        .iter()
        .map(|(base, record)| (*base, record.clone()))
        .collect()
}

pub fn covering(start: usize, len: usize) -> Vec<(usize, MapRecord)> {
    let end = start + len;
    REGISTRY
        .lock()
        .expect("map registry")
        .range(..end)
        .filter(|(base, record)| **base + record.len > start)
        .map(|(base, record)| (*base, record.clone()))
        .collect()
}

pub fn find(address: usize) -> Option<(usize, MapRecord)> {
    covering(address, 1).into_iter().next()
}
