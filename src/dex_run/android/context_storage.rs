use super::*;

impl DexProgram {
    pub(in crate::dex_run) fn context_private_storage_kind(
        &self,
        context: &DexValue,
    ) -> Option<crate::guest_filesystem::PrivateStorageKind> {
        let mut current = context.clone();
        let mut visited = std::collections::HashSet::new();
        loop {
            let key = map_identity_key(&current);
            if !visited.insert(key.clone()) {
                break;
            }
            if let Some(storage) = self
                .runtime
                .borrow()
                .context_storage_kinds
                .get(&key)
                .copied()
            {
                return Some(storage);
            }
            // cite: frameworks-base ContextWrapper.java#getDataDir, getFilesDir, getCacheDir, createDeviceProtectedStorageContext, isDeviceProtectedStorage
            let Some(base) =
                self.read_stored_instance_field(&current, &context_wrapper_base_field())
            else {
                break;
            };
            if !dex_value_is_object_like(&base) {
                break;
            }
            current = base;
        }

        let current_key = map_identity_key(&current);
        let runtime = self.runtime.borrow();
        let is_application = runtime
            .application_instance
            .as_ref()
            .is_some_and(|application| map_identity_key(application) == current_key);
        let is_activity = runtime
            .activity_stack
            .iter()
            .any(|activity| map_identity_key(&activity.receiver) == current_key);
        drop(runtime);
        let is_bootstrap_application = map_identity_key(&self.application_value()) == current_key;
        let is_bootstrap_context = current_key == "reference:Landroid/content/Context;.bootstrap";

        // cite: frameworks-base ContextImpl.java#ContextImpl, createAppContext
        (is_application || is_activity || is_bootstrap_application || is_bootstrap_context)
            .then_some(self.default_private_storage)
    }

    pub(in crate::dex_run) fn create_app_context(&self, application: &DexValue) -> DexValue {
        // cite: frameworks-base ContextImpl.java#createAppContext, setOuterContext, mOuterContext
        let context = self.allocate_object("Landroid/app/ContextImpl;".to_owned());
        self.runtime
            .borrow_mut()
            .context_storage_kinds
            .insert(map_identity_key(&context), self.default_private_storage);
        self.write_stored_instance_field(
            &context,
            &context_impl_outer_context_field(),
            application.clone(),
        );

        context
    }

    pub(in crate::dex_run) fn create_private_storage_context(
        &self,
        source: &DexValue,
        storage: crate::guest_filesystem::PrivateStorageKind,
    ) -> Option<DexValue> {
        self.context_private_storage_kind(source)?;
        // cite: frameworks-base ContextImpl.java#createDeviceProtectedStorageContext, createCredentialProtectedStorageContext
        let context = self.allocate_object("Landroid/app/ContextImpl;".to_owned());
        self.runtime
            .borrow_mut()
            .context_storage_kinds
            .insert(map_identity_key(&context), storage);

        Some(context)
    }

    pub(in crate::dex_run) fn context_is_private_storage(
        &self,
        context: &DexValue,
        storage: crate::guest_filesystem::PrivateStorageKind,
    ) -> Option<i32> {
        // cite: frameworks-base ContextImpl.java#isDeviceProtectedStorage, isCredentialProtectedStorage
        Some(i32::from(
            self.context_private_storage_kind(context)? == storage,
        ))
    }
}

fn context_wrapper_base_field() -> DexFieldId {
    DexFieldId {
        class_descriptor: "Landroid/content/ContextWrapper;".to_owned(),
        type_descriptor: "Landroid/content/Context;".to_owned(),
        name: "mBase".to_owned(),
    }
}

fn context_impl_outer_context_field() -> DexFieldId {
    DexFieldId {
        class_descriptor: "Landroid/app/ContextImpl;".to_owned(),
        type_descriptor: "Landroid/content/Context;".to_owned(),
        name: "mOuterContext".to_owned(),
    }
}
