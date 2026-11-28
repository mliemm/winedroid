use super::*;
use std::collections::BTreeMap;

// cite: libcore HashMap.java#DEFAULT_INITIAL_CAPACITY, MAXIMUM_CAPACITY, DEFAULT_LOAD_FACTOR, TREEIFY_THRESHOLD, MIN_TREEIFY_CAPACITY
const DEFAULT_INITIAL_CAPACITY: i32 = 1 << 4;
const MAXIMUM_CAPACITY: i32 = 1 << 30;
pub(super) const DEFAULT_LOAD_FACTOR: f32 = 0.75;
const TREEIFY_THRESHOLD: usize = 8;
const MIN_TREEIFY_CAPACITY: usize = 64;

pub(super) const HASH_MAP: &str = "Ljava/util/HashMap;";

#[derive(Debug, Clone)]
struct HashTable {
    length: usize,
    bins: BTreeMap<usize, Vec<String>>,
}

impl HashTable {
    fn new(length: usize) -> Self {
        Self {
            length,
            bins: BTreeMap::new(),
        }
    }

    fn index(&self, hash: i32) -> usize {
        (hash as u32 as usize) & (self.length - 1)
    }
}

#[derive(Debug, Clone)]
pub(super) struct HashMapTable {
    pub(super) load_factor: f32,
    pub(super) threshold: i32,
    table: Option<HashTable>,
    hashes: HashMap<String, i32>,
    unordered: Option<String>,
    unmodeled: Option<String>,
}

// cite: libcore HashMap.java#hash
pub(super) fn spread_hash(hash_code: i32) -> i32 {
    hash_code ^ ((hash_code as u32) >> 16) as i32
}

// cite: libcore HashMap.java#tableSizeFor
pub(super) fn table_size_for(cap: i32) -> i32 {
    let shift = cap.wrapping_sub(1).leading_zeros() & 31;
    let n = (u32::MAX >> shift) as i32;
    if n < 0 {
        1
    } else if n >= MAXIMUM_CAPACITY {
        MAXIMUM_CAPACITY
    } else {
        n + 1
    }
}

// cite: libcore Integer.java#hashCode
// cite: libcore Short.java#hashCode
// cite: libcore Byte.java#hashCode
// cite: libcore Character.java#hashCode
// cite: libcore Long.java#hashCode
// cite: libcore Boolean.java#hashCode
// cite: libcore Float.java#hashCode, floatToIntBits
// cite: libcore Double.java#hashCode, doubleToLongBits
fn boxed_hash_code(reference: &str) -> Option<i32> {
    let (descriptor, payload) = reference.split_once(':')?;
    let long_hash = |bits: i64| (bits ^ ((bits as u64) >> 32) as i64) as i32;
    match descriptor {
        "Ljava/lang/Integer;" | "Ljava/lang/Short;" | "Ljava/lang/Byte;" => payload.parse().ok(),
        "Ljava/lang/Character;" => payload.parse::<u16>().ok().map(i32::from),
        "Ljava/lang/Long;" => payload.parse::<i64>().ok().map(long_hash),
        "Ljava/lang/Boolean;" => match payload {
            "true" => Some(1231),
            "false" => Some(1237),
            _ => None,
        },
        "Ljava/lang/Float;" => {
            let value = f32::from_bits(u32::from_str_radix(payload, 16).ok()?);
            Some(if value.is_nan() {
                0x7fc0_0000
            } else {
                value.to_bits() as i32
            })
        }
        "Ljava/lang/Double;" => {
            let value = f64::from_bits(u64::from_str_radix(payload, 16).ok()?);
            let bits = if value.is_nan() {
                0x7ff8_0000_0000_0000
            } else {
                value.to_bits() as i64
            };
            Some(long_hash(bits))
        }
        _ => None,
    }
}

impl HashMapTable {
    // cite: libcore HashMap.java#HashMap
    pub(super) fn new_default() -> Self {
        Self {
            load_factor: DEFAULT_LOAD_FACTOR,
            threshold: 0,
            table: None,
            hashes: HashMap::new(),
            unordered: None,
            unmodeled: None,
        }
    }

    // cite: libcore HashMap.java#HashMap
    pub(super) fn with_initial_capacity(
        initial_capacity: i32,
        load_factor: f32,
    ) -> std::result::Result<Self, String> {
        if initial_capacity < 0 {
            return Err(format!("Illegal initial capacity: {initial_capacity}"));
        }
        let initial_capacity = initial_capacity.min(MAXIMUM_CAPACITY);
        if load_factor <= 0.0 || load_factor.is_nan() {
            return Err(format!(
                "Illegal load factor: {}",
                java_float_to_string(load_factor).unwrap_or_default()
            ));
        }
        Ok(Self {
            load_factor,
            threshold: table_size_for(initial_capacity),
            ..Self::new_default()
        })
    }

    // cite: libcore HashMap.java#readObject
    pub(super) fn for_read_object(load_factor: f32, mappings: i32) -> Self {
        let mut table = Self {
            load_factor,
            ..Self::new_default()
        };
        if mappings > 0 {
            let lf = load_factor.max(0.25).min(4.0);
            let fc = mappings as f32 / lf + 1.0;
            let cap = if fc < DEFAULT_INITIAL_CAPACITY as f32 {
                DEFAULT_INITIAL_CAPACITY
            } else if fc >= MAXIMUM_CAPACITY as f32 {
                MAXIMUM_CAPACITY
            } else {
                table_size_for(fc as i32)
            };
            let ft = cap as f32 * lf;
            table.threshold = if cap < MAXIMUM_CAPACITY && ft < MAXIMUM_CAPACITY as f32 {
                ft as i32
            } else {
                i32::MAX
            };
            table.table = Some(HashTable::new(cap as usize));
        }
        table
    }

    // cite: libcore HashMap.java#capacity
    pub(super) fn capacity(&self) -> i32 {
        match &self.table {
            Some(table) => table.length as i32,
            None if self.threshold > 0 => self.threshold,
            None => DEFAULT_INITIAL_CAPACITY,
        }
    }

    pub(super) fn size(&self) -> usize {
        self.hashes.len()
    }

    pub(super) fn unmodeled(&self) -> Option<&str> {
        self.unmodeled.as_deref()
    }

    pub(super) fn mark_unmodeled(&mut self, reason: String) {
        self.unmodeled.get_or_insert(reason);
    }

    // cite: libcore HashMap.java#resize
    fn resize(&mut self) {
        let old_cap = self.table.as_ref().map_or(0, |table| table.length) as i32;
        let old_thr = self.threshold;
        let mut new_thr = 0;
        let new_cap;
        if old_cap > 0 {
            if old_cap >= MAXIMUM_CAPACITY {
                self.threshold = i32::MAX;
                return;
            }
            new_cap = old_cap << 1;
            if new_cap < MAXIMUM_CAPACITY && old_cap >= DEFAULT_INITIAL_CAPACITY {
                new_thr = old_thr.wrapping_shl(1);
            }
        } else if old_thr > 0 {
            new_cap = old_thr;
        } else {
            new_cap = DEFAULT_INITIAL_CAPACITY;
            new_thr = (DEFAULT_LOAD_FACTOR * DEFAULT_INITIAL_CAPACITY as f32) as i32;
        }
        if new_thr == 0 {
            let ft = new_cap as f32 * self.load_factor;
            new_thr = if new_cap < MAXIMUM_CAPACITY && ft < MAXIMUM_CAPACITY as f32 {
                ft as i32
            } else {
                i32::MAX
            };
        }
        self.threshold = new_thr;
        let mut new_table = HashTable::new(new_cap as usize);
        let old_bins = self
            .table
            .take()
            .map(|table| table.bins)
            .unwrap_or_default();
        for entry_key in old_bins.into_values().flatten() {
            let index = new_table.index(self.hashes[&entry_key]);
            new_table.bins.entry(index).or_default().push(entry_key);
        }
        self.table = Some(new_table);
    }

    // cite: libcore HashMap.java#putVal, treeifyBin
    pub(super) fn insert_new(&mut self, entry_key: String, hash: i32) {
        if self.table.as_ref().is_none_or(|table| table.length == 0) {
            self.resize();
        }
        let table = self.table.as_mut().expect("resize allocates the table");
        let length = table.length;
        let bin = table.bins.entry(table.index(hash)).or_default();
        let bin_count = bin.len();
        bin.push(entry_key.clone());
        self.hashes.insert(entry_key, hash);
        if bin_count >= TREEIFY_THRESHOLD {
            if length < MIN_TREEIFY_CAPACITY {
                self.resize();
            } else {
                self.unordered.get_or_insert(format!(
                    "HashMap tree bin iteration order ({} colliding keys)",
                    bin_count + 1
                ));
            }
        }
        if self.size() as i64 > i64::from(self.threshold) {
            self.resize();
        }
    }

    // cite: libcore HashMap.java#removeNode
    pub(super) fn remove(&mut self, entry_key: &str) {
        let Some(table) = self.table.as_mut() else {
            return;
        };
        let Some(hash) = self.hashes.remove(entry_key) else {
            return;
        };
        let index = table.index(hash);
        if let Some(bin) = table.bins.get_mut(&index) {
            bin.retain(|key| key != entry_key);
            if bin.is_empty() {
                table.bins.remove(&index);
            }
        }
        if self.hashes.is_empty() {
            self.unordered = None;
        }
    }

    // cite: libcore HashMap.java#clear
    pub(super) fn clear(&mut self) {
        if let Some(table) = self.table.as_mut() {
            table.bins.clear();
            self.hashes.clear();
            self.unordered = None;
        }
    }

    // cite: libcore HashMap.java#putMapEntries
    pub(super) fn presize_for(&mut self, incoming: usize) {
        if incoming == 0 {
            return;
        }
        match &self.table {
            None => {
                let ft = incoming as f32 / self.load_factor + 1.0;
                let t = if ft < MAXIMUM_CAPACITY as f32 {
                    ft as i32
                } else {
                    MAXIMUM_CAPACITY
                };
                if t > self.threshold {
                    self.threshold = table_size_for(t);
                }
            }
            Some(_) => {
                while incoming as i64 > i64::from(self.threshold)
                    && self.table.as_ref().map_or(0, |table| table.length)
                        < MAXIMUM_CAPACITY as usize
                {
                    self.resize();
                }
            }
        }
    }

    // cite: libcore HashMap.java#putMapEntries, putVal, resize
    pub(super) fn mark_unordered_if_any_share_a_bin(
        &mut self,
        entry_keys: &[String],
        reason: String,
    ) {
        let Some(table) = &self.table else {
            return;
        };
        let mut bins = HashSet::new();
        for entry_key in entry_keys {
            if let Some(hash) = self.hashes.get(entry_key) {
                if !bins.insert(table.index(*hash)) {
                    self.unordered.get_or_insert(reason);
                    return;
                }
            }
        }
    }

    fn stored_hash(&self, entry_key: &str) -> Option<i32> {
        self.hashes.get(entry_key).copied()
    }

    // cite: libcore HashMap.java#HashIterator, nextNode
    pub(super) fn iteration_order(&self) -> std::result::Result<Vec<&str>, String> {
        if let Some(reason) = self.unmodeled.as_ref().or(self.unordered.as_ref()) {
            return Err(reason.clone());
        }
        Ok(self
            .table
            .iter()
            .flat_map(|table| table.bins.values())
            .flatten()
            .map(String::as_str)
            .collect())
    }
}

impl DexProgram {
    // cite: libcore String.java#hashCode
    // cite: libcore HashMap.java#hash
    pub(super) fn hash_map_key_hash(&self, key: &DexValue) -> Option<i32> {
        if matches!(key, DexValue::Null | DexValue::Int(0)) {
            return Some(0);
        }
        if let DexValue::Reference(reference) = key {
            if let Some(hash_code) = boxed_hash_code(reference) {
                return Some(spread_hash(hash_code));
            }
        }
        let is_string = matches!(key, DexValue::String(_))
            || value_class_descriptor(key) == Some("Ljava/lang/String;");
        if !is_string {
            return None;
        }
        let text = self.string_text(key)?;
        let mut hash_code = 0i32;
        for unit in text.encode_utf16() {
            hash_code = hash_code.wrapping_mul(31).wrapping_add(i32::from(unit));
        }
        Some(spread_hash(hash_code))
    }

    pub(super) fn hash_map_install_table(&self, map: &DexValue, table: HashMapTable) {
        self.runtime
            .borrow_mut()
            .hash_map_tables
            .insert(map_identity_key(map), table);
    }

    pub(super) fn hash_map_entry_count(&self, map_key: &str) -> usize {
        self.runtime
            .borrow()
            .map_entries
            .keys()
            .filter(|(entry_map_key, _)| entry_map_key == map_key)
            .count()
    }

    pub(super) fn hash_map_tracks(&self, map: &DexValue) -> bool {
        value_class_descriptor(map) == Some(HASH_MAP)
            || self
                .runtime
                .borrow()
                .hash_map_tables
                .contains_key(&map_identity_key(map))
    }

    // cite: libcore Object.java#hashCode
    // cite: libcore Enum.java#hashCode
    // cite: libcore Class.java#Class
    // cite: libcore HashMap.java#hash
    pub(super) fn hash_map_key_hash_dispatch(
        &self,
        key: &DexValue,
        depth: usize,
    ) -> Result<std::result::Result<Option<i32>, DexValue>> {
        if let Some(hash) = self.hash_map_key_hash(key) {
            return Ok(Ok(Some(hash)));
        }
        let Some(class_descriptor) = value_class_descriptor(key).map(str::to_owned) else {
            return Ok(Ok(None));
        };
        let identity_invocation = DexInvocation {
            kind: DexInvokeKind::Virtual,
            range: false,
            method_index: 0,
            argument_registers: Default::default(),
            arguments: smallvec::smallvec![key.clone()],
        };
        if self.load_class(&class_descriptor).is_none() {
            if matches!(
                class_descriptor.as_str(),
                "Ljava/lang/Object;" | "Ljava/lang/Class;"
            ) {
                return Ok(Ok(self
                    .object_hash_code_value(&identity_invocation)
                    .map(spread_hash)));
            }
            return Ok(Ok(None));
        }
        let method = DexMethodId {
            class_descriptor: class_descriptor.clone(),
            name: "hashCode".to_owned(),
            proto: crate::dex::DexProto {
                shorty: "I".to_owned(),
                return_type: "I".to_owned(),
                parameters: Vec::new(),
            },
        };
        let Some(resolved) =
            self.resolve_exact_method_in_class_hierarchy(&class_descriptor, &method)?
        else {
            let identity = matches!(
                self.first_framework_class(key).as_deref(),
                Some("Ljava/lang/Object;" | "Ljava/lang/Enum;")
            );
            if !identity {
                return Ok(Ok(None));
            }
            return Ok(Ok(self
                .object_hash_code_value(&identity_invocation)
                .map(spread_hash)));
        };
        match self.execute_method(None, &resolved, Some(&[key.clone()]), depth + 1)? {
            DexExecutionResult::Int(hash_code) => Ok(Ok(Some(spread_hash(hash_code)))),
            DexExecutionResult::Throw(exception) => Ok(Err(exception)),
            DexExecutionResult::Parked(execution) => {
                self.discard_parked_execution(&execution);
                Err(WinedroidError::new(format!(
                    "next missing primitive: {class_descriptor}.hashCode() that parks inside HashMap.put"
                )))
            }
            other => Err(WinedroidError::new(format!(
                "next missing primitive: {class_descriptor}.hashCode() inside HashMap.put returned {other:?}"
            ))),
        }
    }

    pub(super) fn hash_map_stored_hash(&self, map_key: &str, entry_key: &str) -> Option<i32> {
        self.runtime
            .borrow()
            .hash_map_tables
            .get(map_key)?
            .stored_hash(entry_key)
    }

    pub(super) fn hash_map_record_insert(
        &self,
        map: &DexValue,
        entry_key: &str,
        key: &DexValue,
        hash: Option<i32>,
    ) {
        let map_key = map_identity_key(map);
        let tracked = self.runtime.borrow().hash_map_tables.contains_key(&map_key);
        if !tracked {
            if value_class_descriptor(map) != Some(HASH_MAP) {
                return;
            }
            let mut table = HashMapTable::new_default();
            if self.hash_map_entry_count(&map_key) != 1 {
                table.mark_unmodeled(
                    "HashMap entries stored before its table was modeled".to_owned(),
                );
            }
            self.runtime
                .borrow_mut()
                .hash_map_tables
                .insert(map_key.clone(), table);
        }
        let mut runtime = self.runtime.borrow_mut();
        let table = runtime
            .hash_map_tables
            .get_mut(&map_key)
            .expect("table was installed above");
        if table.unmodeled().is_some() {
            return;
        }
        match hash {
            Some(hash) => table.insert_new(entry_key.to_owned(), hash),
            None => table.mark_unmodeled(format!(
                "HashMap bucket of a {} key",
                value_class_descriptor(key)
                    .map(str::to_owned)
                    .unwrap_or_else(|| format!("{key:?}"))
            )),
        }
    }

    pub(super) fn hash_map_record_remove(&self, map: &DexValue, entry_key: &str) {
        if let Some(table) = self
            .runtime
            .borrow_mut()
            .hash_map_tables
            .get_mut(&map_identity_key(map))
        {
            table.remove(entry_key);
        }
    }

    pub(super) fn hash_map_record_clear(&self, map: &DexValue) {
        if let Some(table) = self
            .runtime
            .borrow_mut()
            .hash_map_tables
            .get_mut(&map_identity_key(map))
        {
            table.clear();
        }
    }

    pub(super) fn hash_map_record_presize(&self, map: &DexValue, incoming: usize) {
        if let Some(table) = self
            .runtime
            .borrow_mut()
            .hash_map_tables
            .get_mut(&map_identity_key(map))
        {
            table.presize_for(incoming);
        }
    }

    pub(super) fn hash_map_record_fill_order(
        &self,
        map: &DexValue,
        entry_keys: &[String],
        reason: String,
    ) {
        if let Some(table) = self
            .runtime
            .borrow_mut()
            .hash_map_tables
            .get_mut(&map_identity_key(map))
        {
            table.mark_unordered_if_any_share_a_bin(entry_keys, reason);
        }
    }

    pub(super) fn map_has_hash_table(&self, map_key: &str) -> bool {
        self.runtime.borrow().hash_map_tables.contains_key(map_key)
    }

    pub(super) fn stored_map_entries(&self, map_key: &str) -> Vec<(String, DexValue, DexValue)> {
        let runtime = self.runtime.borrow();
        let mut entries = Vec::new();
        for ((entry_map_key, entry_key), value) in &runtime.map_entries {
            if entry_map_key != map_key {
                continue;
            }
            if let Some(key) = runtime
                .map_entry_keys
                .get(&(entry_map_key.clone(), entry_key.clone()))
            {
                entries.push((entry_key.clone(), key.clone(), value.clone()));
            }
        }
        entries.sort_by(|left, right| left.0.cmp(&right.0));
        entries
    }

    // cite: libcore HashMap.java#HashIterator, nextNode, internalWriteEntries
    pub(super) fn map_entries_in_iteration_order(
        &self,
        map_key: &str,
    ) -> std::result::Result<Vec<(String, DexValue, DexValue)>, String> {
        let entries = self.stored_map_entries(map_key);
        let runtime = self.runtime.borrow();
        let Some(table) = runtime.hash_map_tables.get(map_key) else {
            return Ok(entries);
        };
        if entries.len() <= 1 {
            return Ok(entries);
        }
        let order = table.iteration_order()?;
        if order.len() != entries.len() {
            return Err(format!(
                "HashMap table holds {} nodes for {} stored entries",
                order.len(),
                entries.len()
            ));
        }
        let mut by_key: HashMap<&str, &(String, DexValue, DexValue)> = entries
            .iter()
            .map(|entry| (entry.0.as_str(), entry))
            .collect();
        let mut ordered = Vec::with_capacity(order.len());
        for entry_key in order {
            let Some(entry) = by_key.remove(entry_key) else {
                return Err(format!(
                    "HashMap table node {entry_key} has no stored entry"
                ));
            };
            ordered.push(entry.clone());
        }
        Ok(ordered)
    }

    // cite: libcore HashMap.java#writeObject, capacity, internalWriteEntries
    pub(super) fn hash_map_serial_state(
        &self,
        map: &DexValue,
    ) -> std::result::Result<(f32, i32, i32, Vec<(DexValue, DexValue)>), String> {
        let map_key = map_identity_key(map);
        let (load_factor, threshold, buckets, size, unmodeled) = {
            let runtime = self.runtime.borrow();
            let Some(table) = runtime.hash_map_tables.get(&map_key) else {
                return Err("HashMap table of a map built outside HashMap.<init>".to_owned());
            };
            (
                table.load_factor,
                table.threshold,
                table.capacity(),
                table.size(),
                table.unmodeled().map(str::to_owned),
            )
        };
        if let Some(reason) = unmodeled {
            return Err(reason);
        }
        let entries = self.map_entries_in_iteration_order(&map_key)?;
        if entries.len() != size {
            return Err(format!(
                "HashMap table size {size} for {} stored entries",
                entries.len()
            ));
        }
        Ok((
            load_factor,
            threshold,
            buckets,
            entries
                .into_iter()
                .map(|(_, key, value)| (key, value))
                .collect(),
        ))
    }

    pub(super) fn hash_map_view_guard(
        &self,
        method: &DexMethodId,
        invocation: &DexInvocation,
    ) -> Result<()> {
        let view = matches!(
            (method.name.as_str(), method.proto.descriptor().as_str()),
            ("keySet", "()Ljava/util/Set;")
                | ("entrySet", "()Ljava/util/Set;")
                | ("values", "()Ljava/util/Collection;")
        );
        let Some(map) = invocation.arguments.first() else {
            return Ok(());
        };
        let map_key = map_identity_key(map);
        if !view || !self.map_has_hash_table(&map_key) {
            return Ok(());
        }
        match self.map_entries_in_iteration_order(&map_key) {
            Ok(_) => Ok(()),
            Err(reason) => Err(WinedroidError::new(format!(
                "next missing primitive: {reason}"
            ))),
        }
    }

    // cite: libcore HashMap.java#HashMap
    pub(super) fn hash_map_constructor(
        &self,
        method: &DexMethodId,
        invocation: &DexInvocation,
    ) -> Option<DexExecutionResult> {
        let map = invocation.arguments.first()?;
        let table = match (
            method.proto.descriptor().as_str(),
            &invocation.arguments[1..],
        ) {
            ("()V", []) => Ok(HashMapTable::new_default()),
            ("(I)V", [DexValue::Int(capacity)]) => {
                HashMapTable::with_initial_capacity(*capacity, DEFAULT_LOAD_FACTOR)
            }
            ("(IF)V", [DexValue::Int(capacity), load_factor]) => {
                HashMapTable::with_initial_capacity(*capacity, read_float_argument(load_factor)?)
            }
            _ => return None,
        };
        match table {
            Ok(table) => {
                self.hash_map_install_table(map, table);
                Some(DexExecutionResult::Void)
            }
            Err(message) => Some(DexExecutionResult::Throw(
                self.allocate_generated_exception(DexGeneratedException {
                    class_descriptor: "Ljava/lang/IllegalArgumentException;".to_owned(),
                    detail_message: Some(message),
                }),
            )),
        }
    }
}
