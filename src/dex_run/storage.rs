use std::collections::HashMap;
#[cfg(test)]
use std::fs;
#[cfg(test)]
use std::path::{Path, PathBuf};

use super::*;

const SHARED_PREFERENCES_STORE_FILE: &str = "shared_prefs/store.tsv";
const SHARED_PREFERENCES_STORE_HEADER: &str = "winedroid-shared-preferences-v2";
const LEGACY_SHARED_PREFERENCES_STORE_HEADER: &str = "winedroid-shared-preferences-v1";

enum SharedPreferencesStore {
    Guest(crate::guest_filesystem::ResolvedGuestPath),
    #[cfg(test)]
    Host(PathBuf),
}

impl DexProgram {
    pub(super) fn register_component_callback(&self, invocation: &DexInvocation) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let context_key = instance_field_owner_key(&invocation.arguments[0])?;
        let callback = invocation.arguments[1].clone();
        instance_field_owner_key(&callback)?;

        let mut runtime = self.runtime.borrow_mut();
        let callbacks = runtime.component_callbacks.entry(context_key).or_default();

        if !callbacks
            .iter()
            .any(|registered| map_identity_key(registered) == map_identity_key(&callback))
        {
            callbacks.push(callback);
        }

        Some(())
    }

    pub(super) fn unregister_component_callback(&self, invocation: &DexInvocation) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let context_key = instance_field_owner_key(&invocation.arguments[0])?;
        let callback_key = instance_field_owner_key(&invocation.arguments[1])?;

        let mut runtime = self.runtime.borrow_mut();
        let Some(callbacks) = runtime.component_callbacks.get_mut(&context_key) else {
            return Some(());
        };
        callbacks.retain(|callback| {
            instance_field_owner_key(callback).as_deref() != Some(&callback_key)
        });

        if callbacks.is_empty() {
            runtime.component_callbacks.remove(&context_key);
        }

        Some(())
    }

    pub(super) fn default_shared_preferences_reference(&self) -> Option<DexValue> {
        let context = self.active_context_value();
        self.default_shared_preferences_reference_for_context(&context)
    }

    pub(super) fn default_shared_preferences_reference_for_context(
        &self,
        context: &DexValue,
    ) -> Option<DexValue> {
        let package = self.package_name.as_deref()?;
        self.shared_preferences_reference_for_context(context, &format!("{package}_preferences"))
    }

    pub(super) fn shared_preferences_reference(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        if invocation.arguments.len() != 3 {
            return None;
        }
        let context = invocation.arguments.first()?;
        if !dex_value_is_object_like(context) {
            return None;
        }
        let DexValue::String(name) = &invocation.arguments[1] else {
            return None;
        };

        self.shared_preferences_reference_for_context(context, name)
    }

    fn shared_preferences_reference_for_context(
        &self,
        context: &DexValue,
        name: &str,
    ) -> Option<DexValue> {
        let storage = self.context_private_storage_kind(context)?;
        if let Some(filesystem) = &self.guest_filesystem {
            // cite: frameworks-base ContextImpl.java#getSharedPreferences, getSharedPreferencesPath, getPreferencesDir, getDataDir
            let directory = filesystem.private_guest_path(storage, "shared_prefs");
            if let Ok(directory) = filesystem.resolve(
                &directory,
                crate::external_storage::ExternalStorageAccess::Write,
                &self.external_storage_policy,
            ) {
                let _ = directory.mkdir_all();
            }
        }
        let reference_name = shared_preferences_reference_name(storage, name);
        let preferences = DexValue::Reference(std::rc::Rc::from(format!(
            "Landroid/content/SharedPreferences;:{reference_name}"
        )));
        self.runtime
            .borrow_mut()
            .shared_preferences_storage_kinds
            .insert(map_identity_key(&preferences), storage);

        Some(preferences)
    }

    pub(super) fn put_intent_extra(&self, invocation: &DexInvocation) -> Option<DexValue> {
        if invocation.arguments.len() != 3 {
            return None;
        }
        let intent = invocation.arguments[0].clone();
        let DexValue::String(name) = &invocation.arguments[1] else {
            return None;
        };
        let key = (map_identity_key(&intent), name.to_string());
        self.runtime
            .borrow_mut()
            .intent_extras
            .insert(key, invocation.arguments[2].clone());

        Some(intent)
    }

    pub(super) fn get_intent_extra(&self, invocation: &DexInvocation) -> Option<DexValue> {
        if invocation.arguments.len() != 2 {
            return None;
        }
        let DexValue::String(name) = &invocation.arguments[1] else {
            return None;
        };
        let key = (map_identity_key(&invocation.arguments[0]), name.to_string());

        Some(
            self.runtime
                .borrow()
                .intent_extras
                .get(&key)
                .cloned()
                .unwrap_or(DexValue::Null),
        )
    }

    pub(super) fn load_persisted_shared_preferences(&self) -> Result<()> {
        let mut entries = HashMap::new();
        let mut storage_kinds = HashMap::new();
        for storage in [
            crate::guest_filesystem::PrivateStorageKind::CredentialProtected,
            crate::guest_filesystem::PrivateStorageKind::DeviceProtected,
        ] {
            let Some(store) = self.shared_preferences_store(storage)? else {
                continue;
            };
            let contents = match store {
                SharedPreferencesStore::Guest(path) => match path.read() {
                    Ok(bytes) => String::from_utf8(bytes).map_err(|error| {
                        WinedroidError::with_source(
                            "SharedPreferences store is not valid UTF-8",
                            error,
                        )
                    })?,
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                    Err(error) => {
                        return Err(WinedroidError::with_source(
                            "failed to read SharedPreferences store",
                            error,
                        ));
                    }
                },
                #[cfg(test)]
                SharedPreferencesStore::Host(path) => {
                    if !path.exists() {
                        continue;
                    }
                    fs::read_to_string(&path).map_err(|error| {
                        WinedroidError::with_source(
                            format!(
                                "failed to read SharedPreferences store `{}`",
                                path.display()
                            ),
                            error,
                        )
                    })?
                }
            };
            let legacy_store = contents
                .lines()
                .next()
                .is_some_and(|header| header == LEGACY_SHARED_PREFERENCES_STORE_HEADER);
            let stored_entries = parse_shared_preferences_store(&contents)
                .into_iter()
                .map(|((preferences, key), value)| {
                    (
                        (
                            if legacy_store {
                                normalize_legacy_shared_preferences_key(storage, &preferences)
                            } else {
                                preferences
                            },
                            key,
                        ),
                        value,
                    )
                })
                .collect::<HashMap<_, _>>();
            for ((preferences, _), _) in &stored_entries {
                storage_kinds.insert(preferences.clone(), storage);
            }
            entries.extend(stored_entries);
        }

        let mut runtime = self.runtime.borrow_mut();
        runtime.shared_preferences_entries = entries;
        runtime.shared_preferences_storage_kinds = storage_kinds;

        Ok(())
    }

    pub(super) fn shared_preferences_contains(&self, invocation: &DexInvocation) -> Option<i32> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let key =
            self.shared_preferences_entry_key(&invocation.arguments[0], &invocation.arguments[1])?;

        Some(i32::from(
            self.runtime
                .borrow()
                .shared_preferences_entries
                .contains_key(&key),
        ))
    }

    pub(super) fn shared_preferences_edit(&self, invocation: &DexInvocation) -> Option<DexValue> {
        if invocation.arguments.len() != 1 {
            return None;
        }
        if !matches!(
            invocation.arguments[0],
            DexValue::Object(_) | DexValue::Reference(_)
        ) {
            return None;
        }

        let preferences_key = map_identity_key(&invocation.arguments[0]);
        if !self
            .runtime
            .borrow()
            .shared_preferences_storage_kinds
            .contains_key(&preferences_key)
        {
            return None;
        }
        let editor = {
            let mut runtime = self.runtime.borrow_mut();
            let id = runtime.next_object_id;
            runtime.next_object_id += 1;
            DexValue::Reference(std::rc::Rc::from(format!(
                "Landroid/content/SharedPreferences$Editor;:{id}"
            )))
        };
        let editor_key = map_identity_key(&editor);

        self.runtime.borrow_mut().shared_preferences_editors.insert(
            editor_key,
            SharedPreferencesEditorState {
                preferences_key,
                clear_first: false,
                changes: Vec::new(),
            },
        );

        Some(editor)
    }

    pub(super) fn shared_preferences_get_boolean(&self, invocation: &DexInvocation) -> Option<i32> {
        let key =
            self.shared_preferences_entry_key(&invocation.arguments[0], &invocation.arguments[1])?;

        if let Some(DexValue::Int(value)) =
            self.runtime.borrow().shared_preferences_entries.get(&key)
        {
            return Some(*value);
        }

        shared_preferences_default_boolean(invocation)
    }

    pub(super) fn shared_preferences_get_string(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        let key =
            self.shared_preferences_entry_key(&invocation.arguments[0], &invocation.arguments[1])?;

        if let Some(value) = self.runtime.borrow().shared_preferences_entries.get(&key) {
            return match value {
                DexValue::String(value) => Some(DexValue::String(value.clone())),
                DexValue::Null => Some(DexValue::Null),
                DexValue::Uninitialized
                | DexValue::Int(_)
                | DexValue::Long(_)
                | DexValue::Float(_)
                | DexValue::Object(_)
                | DexValue::Reference(_) => None,
            };
        }

        shared_preferences_default_string(invocation)
    }

    pub(super) fn shared_preferences_get_string_set(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        let key =
            self.shared_preferences_entry_key(&invocation.arguments[0], &invocation.arguments[1])?;

        if let Some(value) = self.runtime.borrow().shared_preferences_entries.get(&key) {
            return match value {
                DexValue::Null => shared_preferences_default_string_set(invocation),
                DexValue::Uninitialized
                | DexValue::Int(_)
                | DexValue::Long(_)
                | DexValue::Float(_)
                | DexValue::String(_)
                | DexValue::Object(_)
                | DexValue::Reference(_) => None,
            };
        }

        shared_preferences_default_string_set(invocation)
    }

    pub(super) fn shared_preferences_get_int(&self, invocation: &DexInvocation) -> Option<i32> {
        let key =
            self.shared_preferences_entry_key(&invocation.arguments[0], &invocation.arguments[1])?;

        if let Some(DexValue::Int(value)) =
            self.runtime.borrow().shared_preferences_entries.get(&key)
        {
            return Some(*value);
        }

        shared_preferences_default_int(invocation)
    }

    pub(super) fn shared_preferences_get_long(&self, invocation: &DexInvocation) -> Option<i64> {
        let key =
            self.shared_preferences_entry_key(&invocation.arguments[0], &invocation.arguments[1])?;

        if let Some(DexValue::Long(value)) =
            self.runtime.borrow().shared_preferences_entries.get(&key)
        {
            return Some(*value);
        }

        shared_preferences_default_long(invocation)
    }

    pub(super) fn shared_preferences_editor_put_boolean(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        if invocation.arguments.len() != 3 {
            return None;
        }

        let DexValue::Int(value) = invocation.arguments[2] else {
            return None;
        };

        self.write_shared_preferences_editor_value(invocation, DexValue::Int(value))
    }

    pub(super) fn shared_preferences_editor_put_string(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        if invocation.arguments.len() != 3 {
            return None;
        }

        let value = match &invocation.arguments[2] {
            DexValue::String(value) => DexValue::String(value.clone()),
            DexValue::Null => DexValue::Null,
            DexValue::Uninitialized
            | DexValue::Int(_)
            | DexValue::Long(_)
            | DexValue::Float(_)
            | DexValue::Object(_)
            | DexValue::Reference(_) => return None,
        };

        self.write_shared_preferences_editor_value(invocation, value)
    }

    pub(super) fn shared_preferences_editor_put_int(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        if invocation.arguments.len() != 3 {
            return None;
        }

        let DexValue::Int(value) = invocation.arguments[2] else {
            return None;
        };

        self.write_shared_preferences_editor_value(invocation, DexValue::Int(value))
    }

    pub(super) fn shared_preferences_editor_put_long(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        if invocation.arguments.len() != 4 {
            return None;
        }

        let DexValue::Long(value) = invocation.arguments[2] else {
            return None;
        };
        let DexValue::Long(high_slot_value) = invocation.arguments[3] else {
            return None;
        };

        if value != high_slot_value {
            return None;
        }

        self.write_shared_preferences_editor_value(invocation, DexValue::Long(value))
    }

    pub(super) fn write_shared_preferences_editor_value(
        &self,
        invocation: &DexInvocation,
        value: DexValue,
    ) -> Option<DexValue> {
        let key = shared_preferences_editor_key(&invocation.arguments[1])?;
        let editor = invocation.arguments[0].clone();
        let editor_key = map_identity_key(&editor);

        self.runtime
            .borrow_mut()
            .shared_preferences_editors
            .get_mut(&editor_key)?
            .changes
            .push(SharedPreferencesEditorChange::Put { key, value });

        Some(editor)
    }

    pub(super) fn shared_preferences_editor_remove(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let key = shared_preferences_editor_key(&invocation.arguments[1])?;
        let editor = invocation.arguments[0].clone();
        let editor_key = map_identity_key(&editor);

        self.runtime
            .borrow_mut()
            .shared_preferences_editors
            .get_mut(&editor_key)?
            .changes
            .push(SharedPreferencesEditorChange::Remove { key });

        Some(editor)
    }

    pub(super) fn shared_preferences_editor_clear(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        let editor = invocation.arguments[0].clone();
        let editor_key = map_identity_key(&editor);

        self.runtime
            .borrow_mut()
            .shared_preferences_editors
            .get_mut(&editor_key)?
            .clear_first = true;

        Some(editor)
    }

    pub(super) fn shared_preferences_editor_apply(&self, invocation: &DexInvocation) -> Option<()> {
        let _persisted = self.apply_shared_preferences_editor(invocation)?;

        Some(())
    }

    pub(super) fn shared_preferences_editor_commit(
        &self,
        invocation: &DexInvocation,
    ) -> Option<i32> {
        Some(i32::from(self.apply_shared_preferences_editor(invocation)?))
    }

    fn apply_shared_preferences_editor(&self, invocation: &DexInvocation) -> Option<bool> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        let editor_key = map_identity_key(&invocation.arguments[0]);
        let state = self.take_shared_preferences_editor_state(&editor_key)?;
        let entries = {
            let mut runtime = self.runtime.borrow_mut();

            if state.clear_first {
                runtime
                    .shared_preferences_entries
                    .retain(|(preferences_key, _key), _value| {
                        preferences_key != &state.preferences_key
                    });
            }

            for change in state.changes {
                match change {
                    SharedPreferencesEditorChange::Put { key, value } => {
                        runtime
                            .shared_preferences_entries
                            .insert((state.preferences_key.clone(), key), value);
                    }
                    SharedPreferencesEditorChange::Remove { key } => {
                        runtime
                            .shared_preferences_entries
                            .remove(&(state.preferences_key.clone(), key));
                    }
                }
            }

            runtime.shared_preferences_entries.clone()
        };

        Some(self.persist_shared_preferences_entries(&entries))
    }

    fn take_shared_preferences_editor_state(
        &self,
        editor_key: &str,
    ) -> Option<SharedPreferencesEditorState> {
        let mut runtime = self.runtime.borrow_mut();
        let state = runtime.shared_preferences_editors.get_mut(editor_key)?;
        let applied = state.clone();

        state.clear_first = false;
        state.changes.clear();

        Some(applied)
    }

    fn persist_shared_preferences_entries(
        &self,
        entries: &HashMap<(String, String), DexValue>,
    ) -> bool {
        let storage_kinds = self
            .runtime
            .borrow()
            .shared_preferences_storage_kinds
            .clone();
        if entries
            .keys()
            .any(|(preferences, _)| !storage_kinds.contains_key(preferences))
        {
            return false;
        }
        let credential_entries = entries
            .iter()
            .filter(|((preferences, _), _)| {
                storage_kinds.get(preferences).copied()
                    == Some(crate::guest_filesystem::PrivateStorageKind::CredentialProtected)
            })
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect::<HashMap<_, _>>();
        let device_entries = entries
            .iter()
            .filter(|((preferences, _), _)| {
                storage_kinds.get(preferences).copied()
                    == Some(crate::guest_filesystem::PrivateStorageKind::DeviceProtected)
            })
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect::<HashMap<_, _>>();

        let credential_persisted = self.persist_shared_preferences_store(
            crate::guest_filesystem::PrivateStorageKind::CredentialProtected,
            &credential_entries,
        );
        let device_storage_available = self.guest_filesystem.is_some();
        #[cfg(test)]
        let device_storage_available =
            device_storage_available || self.app_device_data_dir.is_some();
        let device_persisted = if device_storage_available {
            self.persist_shared_preferences_store(
                crate::guest_filesystem::PrivateStorageKind::DeviceProtected,
                &device_entries,
            )
        } else {
            device_entries.is_empty()
        };

        credential_persisted && device_persisted
    }

    fn persist_shared_preferences_store(
        &self,
        storage: crate::guest_filesystem::PrivateStorageKind,
        entries: &HashMap<(String, String), DexValue>,
    ) -> bool {
        let Ok(Some(store)) = self.shared_preferences_store(storage) else {
            return false;
        };
        let contents = serialize_shared_preferences_store(entries);
        match store {
            SharedPreferencesStore::Guest(path) => path.atomic_replace(contents.as_bytes()).is_ok(),
            #[cfg(test)]
            SharedPreferencesStore::Host(path) => {
                persist_host_shared_preferences_store(&path, contents.as_bytes())
            }
        }
    }

    fn shared_preferences_store(
        &self,
        storage: crate::guest_filesystem::PrivateStorageKind,
    ) -> Result<Option<SharedPreferencesStore>> {
        if let Some(filesystem) = &self.guest_filesystem {
            let directory = filesystem.private_guest_path(storage, "shared_prefs");
            filesystem
                .resolve(
                    &directory,
                    crate::external_storage::ExternalStorageAccess::Write,
                    &self.external_storage_policy,
                )?
                .mkdir_all()
                .map_err(|error| {
                    WinedroidError::with_source(
                        "failed to create SharedPreferences directory",
                        error,
                    )
                })?;
            let guest_path = filesystem.private_guest_path(storage, SHARED_PREFERENCES_STORE_FILE);
            let resolved = filesystem.resolve(
                &guest_path,
                crate::external_storage::ExternalStorageAccess::Write,
                &self.external_storage_policy,
            )?;
            return Ok(Some(SharedPreferencesStore::Guest(resolved)));
        }

        #[cfg(test)]
        {
            let data_dir = match storage {
                crate::guest_filesystem::PrivateStorageKind::CredentialProtected => {
                    self._test_app_data_dir.as_ref()
                }
                crate::guest_filesystem::PrivateStorageKind::DeviceProtected => {
                    self.app_device_data_dir.as_ref()
                }
            };
            return Ok(data_dir
                .map(|path| SharedPreferencesStore::Host(shared_preferences_store_path(path))));
        }

        #[cfg(not(test))]
        Ok(None)
    }
}

#[cfg(test)]
fn persist_host_shared_preferences_store(store_path: &Path, contents: &[u8]) -> bool {
    let Some(parent) = store_path.parent() else {
        return false;
    };

    if fs::create_dir_all(parent).is_err() {
        return false;
    }

    let temp_path = store_path.with_extension("tmp");

    if fs::write(&temp_path, contents).is_err() {
        return false;
    }

    replace_shared_preferences_store(&temp_path, store_path)
}

fn shared_preferences_reference_name(
    storage: crate::guest_filesystem::PrivateStorageKind,
    name: &str,
) -> String {
    let domain = match storage {
        crate::guest_filesystem::PrivateStorageKind::CredentialProtected => "ce",
        crate::guest_filesystem::PrivateStorageKind::DeviceProtected => "de",
    };
    let encoded_name = name
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("{domain}:{encoded_name}")
}

fn normalize_legacy_shared_preferences_key(
    storage: crate::guest_filesystem::PrivateStorageKind,
    preferences: &str,
) -> String {
    const PREFIX: &str = "reference:Landroid/content/SharedPreferences;:";
    let Some(name) = preferences.strip_prefix(PREFIX) else {
        return preferences.to_owned();
    };
    format!(
        "{PREFIX}{}",
        shared_preferences_reference_name(storage, name)
    )
}

impl DexProgram {
    fn shared_preferences_entry_key(
        &self,
        preferences: &DexValue,
        key: &DexValue,
    ) -> Option<(String, String)> {
        let preferences_key = map_identity_key(preferences);
        if !self
            .runtime
            .borrow()
            .shared_preferences_storage_kinds
            .contains_key(&preferences_key)
        {
            return None;
        }
        let DexValue::String(key) = key else {
            return None;
        };

        Some((preferences_key, key.clone().to_string()))
    }
}

pub(super) fn shared_preferences_editor_key(key: &DexValue) -> Option<String> {
    let DexValue::String(key) = key else {
        return None;
    };

    Some(key.clone().to_string())
}

pub(super) fn shared_preferences_default_boolean(invocation: &DexInvocation) -> Option<i32> {
    if invocation.arguments.len() != 3 {
        return None;
    }

    let DexValue::Int(default_value) = invocation.arguments[2] else {
        return None;
    };

    Some(default_value)
}

pub(super) fn shared_preferences_default_string(invocation: &DexInvocation) -> Option<DexValue> {
    if invocation.arguments.len() != 3 {
        return None;
    }

    match &invocation.arguments[2] {
        DexValue::String(value) => Some(DexValue::String(value.clone())),
        DexValue::Null => Some(DexValue::Null),
        DexValue::Int(0) => Some(DexValue::Null),
        DexValue::Uninitialized
        | DexValue::Long(_)
        | DexValue::Float(_)
        | DexValue::Object(_)
        | DexValue::Reference(_) => None,
        DexValue::Int(_) => None,
    }
}

pub(super) fn shared_preferences_default_string_set(
    invocation: &DexInvocation,
) -> Option<DexValue> {
    if invocation.arguments.len() != 3 {
        return None;
    }

    match &invocation.arguments[2] {
        DexValue::Null | DexValue::Int(0) => Some(DexValue::Null),
        DexValue::Object(_) | DexValue::Reference(_) => Some(invocation.arguments[2].clone()),
        DexValue::Uninitialized
        | DexValue::Int(_)
        | DexValue::Long(_)
        | DexValue::Float(_)
        | DexValue::String(_) => None,
    }
}

pub(super) fn shared_preferences_default_int(invocation: &DexInvocation) -> Option<i32> {
    if invocation.arguments.len() != 3 {
        return None;
    }

    let DexValue::Int(default_value) = invocation.arguments[2] else {
        return None;
    };

    Some(default_value)
}

pub(super) fn shared_preferences_default_long(invocation: &DexInvocation) -> Option<i64> {
    if invocation.arguments.len() != 4 {
        return None;
    }

    let DexValue::Long(default_value) = invocation.arguments[2] else {
        return None;
    };
    let DexValue::Long(high_slot_value) = invocation.arguments[3] else {
        return None;
    };

    if default_value != high_slot_value {
        return None;
    }

    Some(default_value)
}

#[cfg(test)]
fn shared_preferences_store_path(data_dir: &Path) -> std::path::PathBuf {
    data_dir.join(SHARED_PREFERENCES_STORE_FILE)
}

fn serialize_shared_preferences_store(entries: &HashMap<(String, String), DexValue>) -> String {
    let mut lines = Vec::new();
    lines.push(SHARED_PREFERENCES_STORE_HEADER.to_owned());

    let mut entries = entries.iter().collect::<Vec<_>>();
    entries.sort_by(|left, right| left.0.cmp(right.0));

    for ((preferences_key, key), value) in entries {
        if let Some((kind, encoded_value)) = encode_shared_preferences_value(value) {
            lines.push(format!(
                "{}\t{}\t{}\t{}",
                encode_hex(preferences_key),
                encode_hex(key),
                kind,
                encoded_value
            ));
        }
    }

    lines.push(String::new());
    lines.join("\n")
}

fn parse_shared_preferences_store(contents: &str) -> HashMap<(String, String), DexValue> {
    let mut entries = HashMap::new();
    let mut lines = contents.lines();

    if !matches!(
        lines.next(),
        Some(SHARED_PREFERENCES_STORE_HEADER | LEGACY_SHARED_PREFERENCES_STORE_HEADER)
    ) {
        return entries;
    }

    for line in lines {
        if line.is_empty() {
            continue;
        }

        let parts = line.split('\t').collect::<Vec<_>>();
        if parts.len() != 4 {
            continue;
        }

        let Some(preferences_key) = decode_hex_string(parts[0]) else {
            continue;
        };
        let Some(key) = decode_hex_string(parts[1]) else {
            continue;
        };
        let Some(value) = decode_shared_preferences_value(parts[2], parts[3]) else {
            continue;
        };

        entries.insert((preferences_key, key), value);
    }

    entries
}

fn encode_shared_preferences_value(value: &DexValue) -> Option<(&'static str, String)> {
    match value {
        DexValue::Int(value) => Some(("int", value.to_string())),
        DexValue::Long(value) => Some(("long", value.to_string())),
        DexValue::String(value) => Some(("string", encode_hex(value))),
        DexValue::Null => Some(("null", String::new())),
        DexValue::Uninitialized
        | DexValue::Float(_)
        | DexValue::Object(_)
        | DexValue::Reference(_) => None,
    }
}

fn decode_shared_preferences_value(kind: &str, encoded_value: &str) -> Option<DexValue> {
    match kind {
        "int" => encoded_value.parse::<i32>().ok().map(DexValue::Int),
        "long" => encoded_value.parse::<i64>().ok().map(DexValue::Long),
        "string" => decode_hex_string(encoded_value).map(|value| DexValue::String(value.into())),
        "null" => Some(DexValue::Null),
        _ => None,
    }
}

fn encode_hex(value: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(value.len() * 2);

    for byte in value.as_bytes() {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }

    encoded
}

fn decode_hex_string(value: &str) -> Option<String> {
    if value.len() % 2 != 0 {
        return None;
    }

    let mut bytes = Vec::with_capacity(value.len() / 2);
    for pair in value.as_bytes().chunks_exact(2) {
        let high = decode_hex_nibble(pair[0])?;
        let low = decode_hex_nibble(pair[1])?;
        bytes.push((high << 4) | low);
    }

    String::from_utf8(bytes).ok()
}

fn decode_hex_nibble(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
fn replace_shared_preferences_store(source: &Path, destination: &Path) -> bool {
    if fs::rename(source, destination).is_ok() {
        return true;
    }

    if destination.exists() && fs::remove_file(destination).is_err() {
        return false;
    }

    fs::rename(source, destination).is_ok()
}
