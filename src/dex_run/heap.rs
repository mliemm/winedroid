use super::*;

#[derive(Debug, Clone, PartialEq)]
pub(super) enum ClassInitializationResult {
    Complete,
    Throw(DexValue),
    Parked(DexParkedExecution),
}

#[derive(Debug, Clone, PartialEq)]
pub(super) enum ClassInitializationStart {
    Complete,
    Started,
    Wait(DexParkedExecution),
}

impl DexProgram {
    pub(super) fn allocate_object(&self, class_descriptor: String) -> DexValue {
        let mut runtime = self.runtime.borrow_mut();
        let id = runtime.next_object_id;
        runtime.next_object_id += 1;

        let object = DexObjectRef {
            id,
            class_descriptor: std::rc::Rc::from(class_descriptor),
        };
        runtime
            .instance_fields
            .register_object_id(id, object.class_descriptor.clone().to_string());

        DexValue::Object(object)
    }

    pub(super) fn allocate_generated_exception(
        &self,
        exception: DexGeneratedException,
    ) -> DexValue {
        // cite: art common_throws.cc#ThrowException
        // cite: art thread.cc#ThrowNewException, ThrowNewWrappedException
        let object = self.allocate_object(exception.class_descriptor);
        let Some(detail_message) = exception.detail_message else {
            return object;
        };

        // cite: libcore Throwable.java#Throwable, detailMessage
        let field = DexFieldId {
            class_descriptor: "Ljava/lang/Throwable;".to_owned(),
            type_descriptor: "Ljava/lang/String;".to_owned(),
            name: "detailMessage".to_owned(),
        };
        assert!(self.write_stored_instance_field(
            &object,
            &field,
            DexValue::String(std::rc::Rc::from(detail_message)),
        ));

        object
    }

    pub(super) fn allocate_filled_array(
        &self,
        class_descriptor: String,
        values: Vec<DexValue>,
    ) -> DexValue {
        let array = self.allocate_object(class_descriptor);
        let array_key = map_identity_key(&array);
        let class_descriptor = value_class_descriptor(&array).unwrap_or("");
        let values = values
            .into_iter()
            .map(|value| {
                normalize_array_value_for_descriptor(class_descriptor, value)
                    .unwrap_or(DexValue::Uninitialized)
            })
            .collect();

        self.runtime
            .borrow_mut()
            .array_entries
            .insert(array_key, values);

        array
    }

    pub(super) fn allocate_array(&self, class_descriptor: String, size: i32) -> Option<DexValue> {
        let size = usize::try_from(size).ok()?;
        let default_value = default_array_value(&class_descriptor);
        let array = self.allocate_object(class_descriptor);
        let array_key = map_identity_key(&array);

        self.runtime
            .borrow_mut()
            .array_entries
            .insert(array_key, vec![default_value; size]);

        Some(array)
    }

    pub(super) fn clone_array_object(&self, invocation: &DexInvocation) -> Option<DexValue> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        let receiver = &invocation.arguments[0];
        // cite: art object.cc#Clone
        let descriptor = value_class_descriptor(receiver)?;
        if !descriptor.starts_with('[') {
            return None;
        }

        let array_key = map_identity_key(receiver);
        let values = self
            .runtime
            .borrow()
            .array_entries
            .get(&array_key)
            .cloned()?;

        Some(self.allocate_filled_array(descriptor.to_owned(), values))
    }

    pub(super) fn store_constructor_argument_in_field(
        &self,
        invocation: &DexInvocation,
        class_descriptor: &str,
        field_name: &str,
        field_type: &str,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let field = DexFieldId {
            class_descriptor: class_descriptor.to_owned(),
            type_descriptor: field_type.to_owned(),
            name: field_name.to_owned(),
        };

        self.write_stored_instance_field(
            &invocation.arguments[0],
            &field,
            invocation.arguments[1].clone(),
        )
        .then_some(())
    }

    pub(super) fn read_stored_instance_field(
        &self,
        object: &DexValue,
        field: &DexFieldId,
    ) -> Option<DexValue> {
        let signature = self.field_signature(field);
        self.read_stored_instance_field_by_signature(object, &signature)
    }

    pub(super) fn read_stored_instance_field_by_signature(
        &self,
        object: &DexValue,
        signature: &str,
    ) -> Option<DexValue> {
        let runtime = self.runtime.borrow();
        match object {
            DexValue::Object(object) => runtime
                .instance_fields
                .object_field(object.id, signature)
                .cloned(),
            DexValue::Reference(reference) => runtime
                .reference_fields
                .reference_field(reference, signature)
                .cloned(),
            _ => None,
        }
    }

    pub(super) fn write_stored_instance_field(
        &self,
        object: &DexValue,
        field: &DexFieldId,
        value: DexValue,
    ) -> bool {
        let signature = self.field_signature(field);
        self.write_stored_instance_field_by_signature(object, &signature, value)
    }

    pub(super) fn write_stored_instance_field_by_signature(
        &self,
        object: &DexValue,
        signature: &str,
        value: DexValue,
    ) -> bool {
        let mut runtime = self.runtime.borrow_mut();
        match object {
            DexValue::Object(object) => {
                let mut ic = None;
                runtime
                    .instance_fields
                    .set_object_field_cached(object.id, signature, value, &mut ic);
                true
            }
            DexValue::Reference(reference) => {
                let reference = reference.to_string();
                runtime
                    .reference_fields
                    .set_reference_field(&reference, signature, value);
                true
            }
            _ => false,
        }
    }

    pub(super) fn remove_stored_instance_field(
        &self,
        object: &DexValue,
        field: &DexFieldId,
    ) -> bool {
        let Some(owner_key) = instance_field_owner_key(object) else {
            return false;
        };
        let signature = self.field_signature(field);

        self.runtime
            .borrow_mut()
            .remove_field(&owner_key, &signature);

        true
    }

    pub(super) fn read_bootstrap_instance_field(
        &self,
        object: &DexValue,
        field: &DexFieldId,
    ) -> Option<DexValue> {
        if field.class_descriptor == "Landroid/content/pm/ApplicationInfo;"
            && field.type_descriptor == "Ljava/lang/String;"
            && matches!(object, DexValue::Reference(name) if name.as_ref() == "Landroid/content/pm/ApplicationInfo;.bootstrap")
        {
            if let Some((apk, package)) = self
                .apk_path
                .as_deref()
                .zip(self.package_name.as_deref())
                .filter(|(apk, _)| apk.ends_with("source.apk"))
            {
                // cite: frameworks-base ApplicationInfo.java#sourceDir, publicSourceDir, nativeLibraryDir, primaryCpuAbi
                let value = match field.name.as_str() {
                    "sourceDir" | "publicSourceDir" => {
                        Some(crate::guest_app_paths::base_apk(package))
                    }
                    "nativeLibraryDir" if apk.parent()?.join("native/arm64-v8a").is_dir() => {
                        Some(crate::guest_app_paths::native_library_dir(package))
                    }
                    "primaryCpuAbi" if apk.parent()?.join("native/arm64-v8a").is_dir() => {
                        Some("arm64-v8a".to_owned())
                    }
                    _ => None,
                };
                if let Some(path) = value {
                    return Some(DexValue::String(Rc::from(path)));
                }
            }
        }
        if field.class_descriptor == "Landroid/content/pm/ApplicationInfo;"
            && field.name == "targetSdkVersion"
            && field.type_descriptor == "I"
        {
            // cite: frameworks-base ApplicationInfo.java#targetSdkVersion
            return Some(DexValue::Int(self.target_sdk_version));
        }

        if field.class_descriptor == "Landroid/content/pm/ApplicationInfo;"
            && field.type_descriptor == "Ljava/lang/String;"
            && matches!(
                field.name.as_str(),
                "dataDir" | "credentialProtectedDataDir" | "deviceProtectedDataDir"
            )
        {
            let filesystem = self.guest_filesystem.as_ref()?;
            // cite: frameworks-base ApplicationInfo.java#initForUser, PRIVATE_FLAG_DEFAULT_TO_DEVICE_PROTECTED_STORAGE
            let storage = match field.name.as_str() {
                "dataDir" => self.default_private_storage,
                "credentialProtectedDataDir" => {
                    crate::guest_filesystem::PrivateStorageKind::CredentialProtected
                }
                "deviceProtectedDataDir" => {
                    crate::guest_filesystem::PrivateStorageKind::DeviceProtected
                }
                _ => unreachable!(),
            };
            return Some(DexValue::String(std::rc::Rc::from(
                filesystem.private_guest_path(storage, ""),
            )));
        }

        if let Some(value) = bootstrap_instance_field_for_object(object, field) {
            return Some(value);
        }

        if let Some(value) = self.read_bootstrap_owner_field(object, field) {
            return Some(value);
        }

        let value = bootstrap_instance_field_with_display_size(object, field, self.display_size())?;

        Some(self.scope_bootstrap_instance_field(object, field, value))
    }

    pub(super) fn read_bootstrap_owner_field(
        &self,
        object: &DexValue,
        field: &DexFieldId,
    ) -> Option<DexValue> {
        let DexValue::Reference(reference) = object else {
            return None;
        };
        let owner = self
            .runtime
            .borrow()
            .bootstrap_field_owners
            .get(reference.as_ref())?
            .clone();

        if field.type_descriptor != owner.class_descriptor {
            return None;
        }

        Some(owner.value)
    }

    pub(super) fn scope_bootstrap_instance_field(
        &self,
        object: &DexValue,
        field: &DexFieldId,
        value: DexValue,
    ) -> DexValue {
        let DexValue::Reference(reference) = value else {
            return value;
        };

        if reference != std::rc::Rc::from(field.display_signature())
            || !field.type_descriptor.starts_with('L')
        {
            return DexValue::Reference(reference);
        }

        let scoped_reference = format!(
            "{}:bootstrap-field:{}:{}",
            field.type_descriptor,
            map_identity_key(object),
            field.display_signature()
        );
        let owner = BootstrapFieldOwner {
            value: object.clone(),
            class_descriptor: field.class_descriptor.clone(),
        };

        self.runtime
            .borrow_mut()
            .bootstrap_field_owners
            .insert(scoped_reference.clone(), owner);

        DexValue::Reference(std::rc::Rc::from(scoped_reference))
    }

    pub(super) fn read_stored_static_field(&self, field: &DexFieldId) -> Option<DexValue> {
        self.runtime.borrow().static_fields.get(field).cloned()
    }

    pub(super) fn write_stored_static_field(&self, field: &DexFieldId, value: DexValue) {
        self.runtime.borrow_mut().static_fields.insert(field, value);
    }

    pub(super) fn ensure_class_initialized(
        &self,
        class_descriptor: &str,
        depth: usize,
    ) -> Result<ClassInitializationResult> {
        self.ensure_class_initialized_inner(class_descriptor, depth, &mut HashSet::new())
    }

    pub(super) fn ensure_class_initialized_synchronously(
        &self,
        class_descriptor: &str,
        depth: usize,
    ) -> Result<()> {
        match self.ensure_class_initialized(class_descriptor, depth)? {
            ClassInitializationResult::Complete => Ok(()),
            ClassInitializationResult::Throw(exception) => Err(WinedroidError::new(format!(
                "class initialization threw during synchronous field access\nclass: {class_descriptor}\nexception: {exception}"
            ))),
            ClassInitializationResult::Parked(execution) => {
                self.discard_parked_execution(&execution);
                Err(WinedroidError::new(format!(
                    "class initialization parked during synchronous field access\nclass: {class_descriptor}"
                )))
            }
        }
    }

    pub(super) fn discard_parked_execution(&self, execution: &DexParkedExecution) {
        self.runtime
            .borrow_mut()
            .discard_parked_runtime_actions(execution);
    }

    fn ensure_class_initialized_inner(
        &self,
        class_descriptor: &str,
        depth: usize,
        active_classes: &mut HashSet<String>,
    ) -> Result<ClassInitializationResult> {
        if !active_classes.insert(class_descriptor.to_owned()) {
            return Err(WinedroidError::new(format!(
                "class initialization hierarchy contains a cycle\nclass: {class_descriptor}"
            )));
        }

        let result = match self.begin_class_initialization(class_descriptor) {
            ClassInitializationStart::Complete => Ok(ClassInitializationResult::Complete),
            ClassInitializationStart::Wait(execution) => {
                Ok(ClassInitializationResult::Parked(execution))
            }
            ClassInitializationStart::Started => {
                self.initialize_started_class(class_descriptor, depth, active_classes)
            }
        };
        active_classes.remove(class_descriptor);
        if result.is_err() {
            self.abort_class_initialization(class_descriptor);
        }
        result
    }

    fn initialize_started_class(
        &self,
        class_descriptor: &str,
        depth: usize,
        active_classes: &mut HashSet<String>,
    ) -> Result<ClassInitializationResult> {
        // cite: art class_linker.cc#ClassLinker::InitializeClass
        let access_flags = self
            .load_class(class_descriptor)
            .map(|loaded| loaded.class_def.access_flags)
            .unwrap_or_else(|| bootstrap_class_modifiers(class_descriptor));
        let default_interfaces = if access_flags & JAVA_ACC_INTERFACE == 0 {
            self.default_interface_initialization_order(class_descriptor)
        } else {
            Vec::new()
        };

        if access_flags & JAVA_ACC_INTERFACE == 0 {
            if let Some(superclass) = self.superclass_descriptor(class_descriptor) {
                match self.ensure_class_initialized_inner(&superclass, depth, active_classes)? {
                    ClassInitializationResult::Complete => {}
                    ClassInitializationResult::Throw(exception) => {
                        self.abort_class_initialization(class_descriptor);
                        return Ok(ClassInitializationResult::Throw(exception));
                    }
                    ClassInitializationResult::Parked(mut execution) => {
                        self.append_parked_runtime_action(
                            &mut execution,
                            ParkedRuntimeAction::ContinueClassInitialization(
                                ClassInitializationContinuation {
                                    class_descriptor: class_descriptor.to_owned(),
                                    default_interfaces,
                                    next_interface_index: 0,
                                },
                            ),
                        );
                        return Ok(ClassInitializationResult::Parked(execution));
                    }
                }
            }
        }

        self.continue_class_initialization(
            ClassInitializationContinuation {
                class_descriptor: class_descriptor.to_owned(),
                default_interfaces,
                next_interface_index: 0,
            },
            depth,
            active_classes,
        )
    }

    pub(super) fn resume_class_initialization(
        &self,
        continuation: ClassInitializationContinuation,
        depth: usize,
    ) -> Result<ClassInitializationResult> {
        let class_descriptor = continuation.class_descriptor.clone();
        let mut active_classes = HashSet::from([continuation.class_descriptor.clone()]);
        let result = self.continue_class_initialization(continuation, depth, &mut active_classes);
        if result.is_err() {
            self.abort_class_initialization(&class_descriptor);
        }
        result
    }

    fn continue_class_initialization(
        &self,
        continuation: ClassInitializationContinuation,
        depth: usize,
        active_classes: &mut HashSet<String>,
    ) -> Result<ClassInitializationResult> {
        for interface_index in
            continuation.next_interface_index..continuation.default_interfaces.len()
        {
            let interface_descriptor = &continuation.default_interfaces[interface_index];
            match self.ensure_class_initialized_inner(
                interface_descriptor,
                depth,
                active_classes,
            )? {
                ClassInitializationResult::Complete => {}
                ClassInitializationResult::Throw(exception) => {
                    self.abort_class_initialization(&continuation.class_descriptor);
                    return Ok(ClassInitializationResult::Throw(exception));
                }
                ClassInitializationResult::Parked(mut execution) => {
                    self.append_parked_runtime_action(
                        &mut execution,
                        ParkedRuntimeAction::ContinueClassInitialization(
                            ClassInitializationContinuation {
                                class_descriptor: continuation.class_descriptor,
                                default_interfaces: continuation.default_interfaces,
                                next_interface_index: interface_index + 1,
                            },
                        ),
                    );
                    return Ok(ClassInitializationResult::Parked(execution));
                }
            }
        }

        // cite: art class_linker.cc#ClassLinker::InitializeClass
        self.seed_encoded_static_values(&continuation.class_descriptor);
        self.seed_framework_static_values(&continuation.class_descriptor);
        match self.run_class_initializer(&continuation.class_descriptor, depth)? {
            ClassInitializationResult::Complete => {
                self.finish_class_initialization(&continuation.class_descriptor);
                Ok(ClassInitializationResult::Complete)
            }
            ClassInitializationResult::Throw(exception) => {
                self.abort_class_initialization(&continuation.class_descriptor);
                Ok(ClassInitializationResult::Throw(exception))
            }
            ClassInitializationResult::Parked(mut execution) => {
                self.append_parked_runtime_action(
                    &mut execution,
                    ParkedRuntimeAction::FinishClassInitialization(continuation.class_descriptor),
                );
                Ok(ClassInitializationResult::Parked(execution))
            }
        }
    }

    // cite: frameworks-base Build.java#SUPPORTED_ABIS, getStringList
    fn seed_framework_static_values(&self, class_descriptor: &str) {
        if class_descriptor != "Landroid/os/Build;" {
            return;
        }
        let abi_list =
            crate::native::bionic::props::get("ro.product.cpu.abilist").unwrap_or_default();
        let abis = abi_list
            .split(',')
            .filter(|abi| !abi.is_empty())
            .map(|abi| DexValue::String(std::rc::Rc::from(abi)))
            .collect();
        let array = self.allocate_filled_array("[Ljava/lang/String;".to_owned(), abis);
        let field = DexFieldId {
            class_descriptor: class_descriptor.to_owned(),
            type_descriptor: "[Ljava/lang/String;".to_owned(),
            name: "SUPPORTED_ABIS".to_owned(),
        };

        self.write_stored_static_field(&field, array);
    }

    fn default_interface_initialization_order(&self, class_descriptor: &str) -> Vec<String> {
        let mut interfaces = Vec::new();
        let mut visited_interfaces = HashSet::new();
        for interface in self.class_interface_descriptors(class_descriptor) {
            self.collect_default_interfaces(&interface, &mut visited_interfaces, &mut interfaces);
        }
        interfaces
    }

    fn collect_default_interfaces(
        &self,
        interface_descriptor: &str,
        visited_interfaces: &mut HashSet<String>,
        interfaces: &mut Vec<String>,
    ) {
        if !visited_interfaces.insert(interface_descriptor.to_owned()) {
            return;
        }

        // cite: art class_linker.cc#ClassLinker::InitializeDefaultInterfaceRecursive
        for parent in self.class_interface_descriptors(interface_descriptor) {
            self.collect_default_interfaces(&parent, visited_interfaces, interfaces);
        }

        let declares_default_method = self.load_class(interface_descriptor).is_some_and(|loaded| {
            loaded
                .class_def
                .virtual_methods
                .iter()
                .any(|method| method.code_offset != 0)
        });
        if declares_default_method {
            interfaces.push(interface_descriptor.to_owned());
        }
    }

    pub(super) fn append_parked_runtime_action(
        &self,
        execution: &mut DexParkedExecution,
        action: ParkedRuntimeAction,
    ) {
        let action_id = self
            .runtime
            .borrow_mut()
            .register_parked_runtime_action(action);
        execution.push_runtime_action(action_id);
    }

    pub(super) fn begin_class_initialization(
        &self,
        class_descriptor: &str,
    ) -> ClassInitializationStart {
        // cite: art class_linker.cc#ClassLinker::InitializeClass, ClassLinker::WaitForInitializeClass
        let mut runtime = self.runtime.borrow_mut();
        if runtime.initialized_classes.contains(class_descriptor) {
            return ClassInitializationStart::Complete;
        }
        let current_thread = map_identity_key(&runtime.current_guest_thread);
        if let Some(initializing_thread) = runtime.initializing_classes.get(class_descriptor) {
            if initializing_thread == &current_thread {
                return ClassInitializationStart::Complete;
            }
            return ClassInitializationStart::Wait(DexParkedExecution::waiting_for_class(
                class_descriptor.to_owned(),
            ));
        }
        runtime
            .initializing_classes
            .insert(class_descriptor.to_owned(), current_thread);

        ClassInitializationStart::Started
    }

    pub(super) fn finish_class_initialization(&self, class_descriptor: &str) {
        self.runtime
            .borrow_mut()
            .complete_class_initialization(class_descriptor);
    }

    pub(super) fn abort_class_initialization(&self, class_descriptor: &str) {
        self.runtime
            .borrow_mut()
            .abort_class_initialization(class_descriptor);
    }

    // cite: art class_linker.cc#InitializeClass
    // cite: art dex_file_structs.h#static_values_off_
    pub(super) fn seed_encoded_static_values(&self, class_descriptor: &str) {
        let _ = self.ensure_static_class_layout(class_descriptor);
        let Some(loaded_class) = self.load_class(class_descriptor) else {
            return;
        };

        for (encoded_field, static_value) in loaded_class
            .class_def
            .static_fields
            .iter()
            .zip(loaded_class.class_def.static_values.iter())
        {
            let Ok(field) = self.field_id(loaded_class.dex_file_index, encoded_field.field_index)
            else {
                continue;
            };

            let value = match static_value {
                crate::dex::DexStaticValue::Int(value) => DexValue::Int(*value),
                crate::dex::DexStaticValue::Long(value) => DexValue::Long(*value),
                crate::dex::DexStaticValue::FloatBits(bits) => {
                    DexValue::Float(f32::from_bits(*bits))
                }
                crate::dex::DexStaticValue::DoubleBits(bits) => DexValue::Long(*bits as i64),
                crate::dex::DexStaticValue::String(value) => {
                    DexValue::String(std::rc::Rc::from(value.clone()))
                }
                crate::dex::DexStaticValue::Null => DexValue::Null,
                crate::dex::DexStaticValue::Boolean(value) => DexValue::Int(i32::from(*value)),
                crate::dex::DexStaticValue::Unsupported => continue,
            };

            self.write_stored_static_field(&field, value);
        }
    }

    pub(super) fn run_class_initializer(
        &self,
        class_descriptor: &str,
        depth: usize,
    ) -> Result<ClassInitializationResult> {
        let method = DexMethodId {
            class_descriptor: class_descriptor.to_owned(),
            name: "<clinit>".to_owned(),
            proto: crate::dex::DexProto {
                shorty: "V".to_owned(),
                return_type: "V".to_owned(),
                parameters: Vec::new(),
            },
        };
        let Some(resolved) = self.resolve_exact_method(&method)? else {
            return Ok(ClassInitializationResult::Complete);
        };
        let result = self.execute_method(None, &resolved, None, depth + 1)?;

        match result {
            DexExecutionResult::Void => Ok(ClassInitializationResult::Complete),
            DexExecutionResult::Throw(exception) => Ok(ClassInitializationResult::Throw(exception)),
            DexExecutionResult::Parked(execution) => {
                Ok(ClassInitializationResult::Parked(execution))
            }
            DexExecutionResult::Int(_) | DexExecutionResult::Value(_) => Err(WinedroidError::new(
                "DEX class initializer did not complete normally",
            )),
        }
    }

    pub(super) fn initialize_enum(&self, invocation: &DexInvocation) -> Option<()> {
        if invocation.arguments.len() != 3 {
            return None;
        }

        let enum_key = map_identity_key(&invocation.arguments[0]);
        let class_descriptor = value_class_descriptor(&invocation.arguments[0])?.to_owned();
        let DexValue::String(name) = &invocation.arguments[1] else {
            return None;
        };
        let DexValue::Int(ordinal) = invocation.arguments[2] else {
            return None;
        };

        self.runtime.borrow_mut().enum_values.insert(
            enum_key,
            EnumRuntimeValue {
                class_descriptor,
                name: name.clone().to_string(),
                ordinal,
            },
        );

        Some(())
    }

    pub(super) fn enum_compare_to(&self, arguments: &[DexValue]) -> Option<i32> {
        let left = arguments.first()?;
        let right = arguments.get(1)?;
        let left_class = value_class_descriptor(left)?;
        let right_class = value_class_descriptor(right)?;
        if left_class != right_class {
            return None;
        }

        Some(self.enum_ordinal_value(left)? - self.enum_ordinal_value(right)?)
    }

    pub(super) fn enum_ordinal(&self, invocation: &DexInvocation) -> Option<i32> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        self.enum_ordinal_value(&invocation.arguments[0])
    }

    pub(super) fn enum_name(&self, invocation: &DexInvocation) -> Option<String> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        self.enum_name_value(&invocation.arguments[0])
    }

    pub(super) fn enum_ordinal_value(&self, value: &DexValue) -> Option<i32> {
        let enum_key = map_identity_key(value);

        self.runtime
            .borrow()
            .enum_values
            .get(&enum_key)
            .map(|value| value.ordinal)
            .or_else(|| bootstrap_enum_reference_ordinal(value))
            .or_else(|| lifecycle_state_ordinal(value))
            .or_else(|| lifecycle_event_ordinal(value))
    }

    pub(super) fn enum_name_value(&self, value: &DexValue) -> Option<String> {
        let enum_key = map_identity_key(value);

        if let Some(value) = self.runtime.borrow().enum_values.get(&enum_key) {
            return Some(value.name.clone());
        }

        enum_reference_name(value)
    }

    pub(super) fn enum_value_by_name(
        &self,
        class_descriptor: &str,
        name: &str,
    ) -> Option<DexValue> {
        self.runtime
            .borrow()
            .enum_values
            .iter()
            .find_map(|(key, value)| {
                if value.class_descriptor != class_descriptor || value.name != name {
                    return None;
                }

                object_key_to_value(key, class_descriptor)
            })
    }
}

fn object_key_to_value(key: &str, class_descriptor: &str) -> Option<DexValue> {
    let object_id = key.strip_prefix("object:")?.parse::<u64>().ok()?;

    Some(DexValue::Object(DexObjectRef {
        id: object_id,
        class_descriptor: std::rc::Rc::from(class_descriptor.to_owned()),
    }))
}
