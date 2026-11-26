use super::*;

#[derive(Default)]
pub(in crate::dex_run) struct LruCacheState {
    entries: Vec<(DexValue, DexValue)>,
    size: i32,
    max_size: i32,
    put_count: i32,
    create_count: i32,
    eviction_count: i32,
    hit_count: i32,
    miss_count: i32,
}

type Thrown<T> = std::result::Result<T, DexValue>;

macro_rules! java_try {
    ($expression:expr) => {
        match $expression? {
            Ok(value) => value,
            Err(thrown) => return Ok(Err(thrown)),
        }
    };
}

const LRU_CACHE: &str = "Landroid/util/LruCache;";
const OBJECT: &str = "Ljava/lang/Object;";

impl DexProgram {
    pub(in crate::dex_run) fn invoke_lru_cache_method(
        &self,
        method: &DexMethodId,
        invocation: &DexInvocation,
        depth: usize,
    ) -> Result<Option<DexExecutionResult>> {
        if !self.is_lru_cache_class(&method.class_descriptor) {
            return Ok(None);
        }
        let Some(receiver) = invocation.arguments.first() else {
            return Ok(None);
        };
        let arguments = &invocation.arguments[1..];
        let descriptor = method.proto.descriptor();
        let outcome = match (method.name.as_str(), descriptor.as_str(), arguments) {
            // cite: frameworks-base LruCache.java#LruCache
            ("<init>", "(I)V", [DexValue::Int(max_size)]) => self.lru_init(receiver, *max_size),
            // cite: frameworks-base LruCache.java#get
            ("get", "(Ljava/lang/Object;)Ljava/lang/Object;", [key]) => {
                self.lru_get(receiver, key, depth)
            }
            // cite: frameworks-base LruCache.java#put
            ("put", "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;", [key, value]) => {
                self.lru_put(receiver, key, value, depth)
            }
            // cite: frameworks-base LruCache.java#remove
            ("remove", "(Ljava/lang/Object;)Ljava/lang/Object;", [key]) => {
                self.lru_remove(receiver, key, depth)
            }
            // cite: frameworks-base LruCache.java#trimToSize
            ("trimToSize", "(I)V", [DexValue::Int(max_size)]) => self
                .lru_trim_to_size(receiver, *max_size, depth)
                .map(|result| result.map(|()| DexValue::Null)),
            // cite: frameworks-base LruCache.java#resize
            ("resize", "(I)V", [DexValue::Int(max_size)]) => {
                self.lru_resize(receiver, *max_size, depth)
            }
            // cite: frameworks-base LruCache.java#evictAll
            ("evictAll", "()V", []) => self
                .lru_trim_to_size(receiver, -1, depth)
                .map(|result| result.map(|()| DexValue::Null)),
            // cite: frameworks-base LruCache.java#size, maxSize, hitCount, missCount, createCount, putCount, evictionCount
            (
                "size" | "maxSize" | "hitCount" | "missCount" | "createCount" | "putCount"
                | "evictionCount",
                "()I",
                [],
            ) => return self.lru_counter(receiver, &method.name).map(Some),
            // cite: frameworks-base LruCache.java#sizeOf
            ("sizeOf", "(Ljava/lang/Object;Ljava/lang/Object;)I", [_, _]) => {
                return Ok(Some(DexExecutionResult::Int(1)))
            }
            // cite: frameworks-base LruCache.java#create
            ("create", "(Ljava/lang/Object;)Ljava/lang/Object;", [_]) => {
                return Ok(Some(DexExecutionResult::Value(DexValue::Null)))
            }
            // cite: frameworks-base LruCache.java#entryRemoved
            (
                "entryRemoved",
                "(ZLjava/lang/Object;Ljava/lang/Object;Ljava/lang/Object;)V",
                [_, _, _, _],
            ) => return Ok(Some(DexExecutionResult::Void)),
            _ => return Ok(None),
        }?;

        Ok(Some(match outcome {
            Ok(DexValue::Uninitialized) => DexExecutionResult::Void,
            Ok(value) if method.proto.return_type == "V" => {
                let _ = value;
                DexExecutionResult::Void
            }
            Ok(value) => DexExecutionResult::Value(value),
            Err(thrown) => DexExecutionResult::Throw(thrown),
        }))
    }

    fn is_lru_cache_class(&self, class_descriptor: &str) -> bool {
        let mut current = Some(class_descriptor.to_owned());
        while let Some(descriptor) = current {
            if descriptor == LRU_CACHE {
                return true;
            }
            current = self.superclass_descriptor(&descriptor);
        }

        false
    }

    fn lru_exception(&self, class_descriptor: &str, message: String) -> DexValue {
        self.allocate_generated_exception(DexGeneratedException {
            class_descriptor: class_descriptor.to_owned(),
            detail_message: Some(message),
        })
    }

    fn lru_init(&self, receiver: &DexValue, max_size: i32) -> Result<Thrown<DexValue>> {
        if max_size <= 0 {
            return Ok(Err(self.lru_exception(
                "Ljava/lang/IllegalArgumentException;",
                "maxSize <= 0".to_owned(),
            )));
        }
        self.runtime.borrow_mut().lru_caches.insert(
            map_identity_key(receiver),
            LruCacheState {
                max_size,
                ..LruCacheState::default()
            },
        );

        Ok(Ok(DexValue::Null))
    }

    fn lru_state<T>(
        &self,
        receiver: &DexValue,
        read: impl FnOnce(&mut LruCacheState) -> T,
    ) -> Result<T> {
        let mut runtime = self.runtime.borrow_mut();
        let Some(state) = runtime.lru_caches.get_mut(&map_identity_key(receiver)) else {
            return Err(WinedroidError::new(format!(
                "LruCache used before <init>\nreceiver: {receiver}\nnext missing primitive: LruCache.<init> on this receiver"
            )));
        };

        Ok(read(state))
    }

    fn lru_counter(&self, receiver: &DexValue, name: &str) -> Result<DexExecutionResult> {
        let value = self.lru_state(receiver, |state| match name {
            "size" => state.size,
            "maxSize" => state.max_size,
            "hitCount" => state.hit_count,
            "missCount" => state.miss_count,
            "createCount" => state.create_count,
            "putCount" => state.put_count,
            _ => state.eviction_count,
        })?;

        Ok(DexExecutionResult::Int(value))
    }

    fn lru_keys_equal(&self, left: &DexValue, right: &DexValue, depth: usize) -> Result<bool> {
        let is_plain = |value: &DexValue| {
            matches!(
                value,
                DexValue::String(_) | DexValue::Int(_) | DexValue::Long(_) | DexValue::Float(_)
            )
        };
        if is_plain(left) && is_plain(right) {
            return Ok(left == right);
        }

        self.runtime_values_equal(left, right, depth + 1, true)
    }

    fn lru_find(&self, receiver: &DexValue, key: &DexValue, depth: usize) -> Result<Option<usize>> {
        let keys = self.lru_state(receiver, |state| {
            state
                .entries
                .iter()
                .map(|(entry_key, _)| entry_key.clone())
                .collect::<Vec<_>>()
        })?;
        for (index, entry_key) in keys.iter().enumerate() {
            if self.lru_keys_equal(key, entry_key, depth + 1)? {
                return Ok(Some(index));
            }
        }

        Ok(None)
    }

    // cite: libcore LinkedHashMap.java#put, get, afterNodeAccess
    fn lru_map_put(
        &self,
        receiver: &DexValue,
        key: &DexValue,
        value: &DexValue,
        depth: usize,
    ) -> Result<Option<DexValue>> {
        let index = self.lru_find(receiver, key, depth)?;
        self.lru_state(receiver, |state| match index {
            Some(index) if index < state.entries.len() => {
                let (entry_key, previous) = state.entries.remove(index);
                state.entries.push((entry_key, value.clone()));
                Some(previous)
            }
            Some(_) => {
                state.entries.push((key.clone(), value.clone()));
                None
            }
            None => {
                state.entries.push((key.clone(), value.clone()));
                None
            }
        })
    }

    fn lru_map_get(
        &self,
        receiver: &DexValue,
        key: &DexValue,
        depth: usize,
    ) -> Result<Option<DexValue>> {
        let Some(index) = self.lru_find(receiver, key, depth)? else {
            return Ok(None);
        };
        self.lru_state(receiver, |state| {
            if index >= state.entries.len() {
                return None;
            }
            let entry = state.entries.remove(index);
            let value = entry.1.clone();
            state.entries.push(entry);
            Some(value)
        })
    }

    fn lru_map_remove(
        &self,
        receiver: &DexValue,
        key: &DexValue,
        depth: usize,
    ) -> Result<Option<DexValue>> {
        let Some(index) = self.lru_find(receiver, key, depth)? else {
            return Ok(None);
        };

        self.lru_state(receiver, |state| {
            (index < state.entries.len()).then(|| state.entries.remove(index).1)
        })
    }

    pub(in crate::dex_run) fn guest_override(
        &self,
        receiver: &DexValue,
        name: &str,
        return_type: &str,
        parameters: &[&str],
        arguments: Vec<DexValue>,
        depth: usize,
    ) -> Result<Option<DexExecutionResult>> {
        let Some(class_descriptor) = value_class_descriptor(receiver).map(str::to_owned) else {
            return Ok(None);
        };
        let hook = guest_hook_method(&class_descriptor, name, return_type, parameters);
        let Some(resolved) =
            self.resolve_exact_method_in_class_hierarchy(&class_descriptor, &hook)?
        else {
            return Ok(None);
        };
        let mut hook_arguments = vec![receiver.clone()];
        hook_arguments.extend(arguments);
        let result = self.execute_method(None, &resolved, Some(&hook_arguments), depth + 1)?;
        if let DexExecutionResult::Parked(execution) = &result {
            self.discard_parked_execution(execution);
            return Err(WinedroidError::new(format!(
                "guest override {name} did not complete normally"
            )));
        }

        Ok(Some(result))
    }

    // cite: frameworks-base LruCache.java#safeSizeOf, sizeOf
    fn lru_safe_size_of(
        &self,
        receiver: &DexValue,
        key: &DexValue,
        value: &DexValue,
        depth: usize,
    ) -> Result<Thrown<i32>> {
        let result = self.guest_override(
            receiver,
            "sizeOf",
            "I",
            &[OBJECT, OBJECT],
            vec![key.clone(), value.clone()],
            depth,
        )?;
        let size = match result {
            None => 1,
            Some(DexExecutionResult::Int(size)) => size,
            Some(DexExecutionResult::Throw(thrown)) => return Ok(Err(thrown)),
            Some(_) => {
                return Err(WinedroidError::new(
                    "LruCache.sizeOf(Object, Object) did not return an int",
                ))
            }
        };
        if size < 0 {
            let message = format!(
                "Negative size: {}={}",
                self.lru_display(key)?,
                self.lru_display(value)?
            );
            return Ok(Err(
                self.lru_exception("Ljava/lang/IllegalStateException;", message)
            ));
        }

        Ok(Ok(size))
    }

    // cite: frameworks-base LruCache.java#entryRemoved
    fn lru_entry_removed(
        &self,
        receiver: &DexValue,
        evicted: bool,
        key: &DexValue,
        old_value: &DexValue,
        new_value: &DexValue,
        depth: usize,
    ) -> Result<Thrown<()>> {
        let result = self.guest_override(
            receiver,
            "entryRemoved",
            "V",
            &["Z", OBJECT, OBJECT, OBJECT],
            vec![
                DexValue::Int(i32::from(evicted)),
                key.clone(),
                old_value.clone(),
                new_value.clone(),
            ],
            depth,
        )?;
        match result {
            Some(DexExecutionResult::Throw(thrown)) => Ok(Err(thrown)),
            _ => Ok(Ok(())),
        }
    }

    // cite: frameworks-base LruCache.java#create
    fn lru_create(
        &self,
        receiver: &DexValue,
        key: &DexValue,
        depth: usize,
    ) -> Result<Thrown<DexValue>> {
        let result = self.guest_override(
            receiver,
            "create",
            OBJECT,
            &[OBJECT],
            vec![key.clone()],
            depth,
        )?;
        match result {
            None => Ok(Ok(DexValue::Null)),
            Some(DexExecutionResult::Value(value)) => Ok(Ok(value)),
            Some(DexExecutionResult::Throw(thrown)) => Ok(Err(thrown)),
            Some(_) => Err(WinedroidError::new(
                "LruCache.create(Object) did not return an object",
            )),
        }
    }

    fn lru_display(&self, value: &DexValue) -> Result<String> {
        match value {
            DexValue::String(text) => Ok(text.to_string()),
            DexValue::Null => Ok("null".to_owned()),
            _ => self.object_value_to_string(value).ok_or_else(|| {
                WinedroidError::new(format!(
                    "LruCache.safeSizeOf could not stringify an entry\nvalue: {value}\nnext missing primitive: Object.toString for the entry"
                ))
            }),
        }
    }

    fn lru_adjust_size(&self, receiver: &DexValue, delta: i32) -> Result<()> {
        self.lru_state(receiver, |state| {
            state.size = state.size.wrapping_add(delta)
        })
    }

    fn lru_is_null(value: &DexValue) -> bool {
        matches!(value, DexValue::Null | DexValue::Int(0))
    }

    fn lru_get(
        &self,
        receiver: &DexValue,
        key: &DexValue,
        depth: usize,
    ) -> Result<Thrown<DexValue>> {
        if Self::lru_is_null(key) {
            return Ok(Err(self.lru_exception(
                "Ljava/lang/NullPointerException;",
                "key == null".to_owned(),
            )));
        }
        if let Some(value) = self.lru_map_get(receiver, key, depth)? {
            self.lru_state(receiver, |state| {
                state.hit_count = state.hit_count.wrapping_add(1)
            })?;
            return Ok(Ok(value));
        }
        self.lru_state(receiver, |state| {
            state.miss_count = state.miss_count.wrapping_add(1)
        })?;

        let created = java_try!(self.lru_create(receiver, key, depth));
        if Self::lru_is_null(&created) {
            return Ok(Ok(DexValue::Null));
        }
        self.lru_state(receiver, |state| {
            state.create_count = state.create_count.wrapping_add(1)
        })?;
        let mut previous = self.lru_map_put(receiver, key, &created, depth)?;
        if let Some(existing) = &previous {
            self.lru_map_put(receiver, key, existing, depth)?;
        } else {
            let size = java_try!(self.lru_safe_size_of(receiver, key, &created, depth));
            self.lru_adjust_size(receiver, size)?;
        }

        match previous.take() {
            Some(existing) => {
                java_try!(self.lru_entry_removed(receiver, false, key, &created, &existing, depth));
                Ok(Ok(existing))
            }
            None => {
                let max_size = self.lru_state(receiver, |state| state.max_size)?;
                java_try!(self.lru_trim_to_size(receiver, max_size, depth));
                Ok(Ok(created))
            }
        }
    }

    fn lru_put(
        &self,
        receiver: &DexValue,
        key: &DexValue,
        value: &DexValue,
        depth: usize,
    ) -> Result<Thrown<DexValue>> {
        if Self::lru_is_null(key) || Self::lru_is_null(value) {
            return Ok(Err(self.lru_exception(
                "Ljava/lang/NullPointerException;",
                "key == null || value == null".to_owned(),
            )));
        }
        self.lru_state(receiver, |state| {
            state.put_count = state.put_count.wrapping_add(1)
        })?;
        let size = java_try!(self.lru_safe_size_of(receiver, key, value, depth));
        self.lru_adjust_size(receiver, size)?;
        let previous = self.lru_map_put(receiver, key, value, depth)?;
        if let Some(previous) = &previous {
            let previous_size = java_try!(self.lru_safe_size_of(receiver, key, previous, depth));
            self.lru_adjust_size(receiver, previous_size.wrapping_neg())?;
            java_try!(self.lru_entry_removed(receiver, false, key, previous, value, depth));
        }
        let max_size = self.lru_state(receiver, |state| state.max_size)?;
        java_try!(self.lru_trim_to_size(receiver, max_size, depth));

        Ok(Ok(previous.unwrap_or(DexValue::Null)))
    }

    fn lru_remove(
        &self,
        receiver: &DexValue,
        key: &DexValue,
        depth: usize,
    ) -> Result<Thrown<DexValue>> {
        if Self::lru_is_null(key) {
            return Ok(Err(self.lru_exception(
                "Ljava/lang/NullPointerException;",
                "key == null".to_owned(),
            )));
        }
        let Some(previous) = self.lru_map_remove(receiver, key, depth)? else {
            return Ok(Ok(DexValue::Null));
        };
        let size = java_try!(self.lru_safe_size_of(receiver, key, &previous, depth));
        self.lru_adjust_size(receiver, size.wrapping_neg())?;
        java_try!(self.lru_entry_removed(receiver, false, key, &previous, &DexValue::Null, depth));

        Ok(Ok(previous))
    }

    fn lru_resize(
        &self,
        receiver: &DexValue,
        max_size: i32,
        depth: usize,
    ) -> Result<Thrown<DexValue>> {
        if max_size <= 0 {
            return Ok(Err(self.lru_exception(
                "Ljava/lang/IllegalArgumentException;",
                "maxSize <= 0".to_owned(),
            )));
        }
        self.lru_state(receiver, |state| state.max_size = max_size)?;
        java_try!(self.lru_trim_to_size(receiver, max_size, depth));

        Ok(Ok(DexValue::Null))
    }

    fn lru_trim_to_size(
        &self,
        receiver: &DexValue,
        max_size: i32,
        depth: usize,
    ) -> Result<Thrown<()>> {
        loop {
            let (size, eldest) = self.lru_state(receiver, |state| {
                (state.size, state.entries.first().cloned())
            })?;
            if size < 0 || (eldest.is_none() && size != 0) {
                let class_name = value_class_descriptor(receiver)
                    .unwrap_or(LRU_CACHE)
                    .trim_start_matches('L')
                    .trim_end_matches(';')
                    .replace('/', ".");
                return Ok(Err(self.lru_exception(
                    "Ljava/lang/IllegalStateException;",
                    format!("{class_name}.sizeOf() is reporting inconsistent results!"),
                )));
            }
            if size <= max_size {
                return Ok(Ok(()));
            }
            let Some((key, value)) = eldest else {
                return Ok(Ok(()));
            };
            self.lru_state(receiver, |state| {
                state.entries.remove(0);
            })?;
            let removed_size = java_try!(self.lru_safe_size_of(receiver, &key, &value, depth));
            self.lru_state(receiver, |state| {
                state.size = state.size.wrapping_sub(removed_size);
                state.eviction_count = state.eviction_count.wrapping_add(1);
            })?;
            java_try!(self.lru_entry_removed(receiver, true, &key, &value, &DexValue::Null, depth));
        }
    }
}

pub(in crate::dex_run) fn guest_hook_method(
    class_descriptor: &str,
    name: &str,
    return_type: &str,
    parameters: &[&str],
) -> DexMethodId {
    let shorty_of = |descriptor: &str| {
        if descriptor.starts_with('L') || descriptor.starts_with('[') {
            'L'
        } else {
            descriptor.chars().next().unwrap_or('V')
        }
    };

    DexMethodId {
        class_descriptor: class_descriptor.to_owned(),
        name: name.to_owned(),
        proto: crate::dex::DexProto {
            shorty: std::iter::once(return_type)
                .chain(parameters.iter().copied())
                .map(shorty_of)
                .collect(),
            return_type: return_type.to_owned(),
            parameters: parameters.iter().map(|value| (*value).to_owned()).collect(),
        },
    }
}
