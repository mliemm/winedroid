use super::super::*;

// cite: frameworks-base Context.java#MODE_WORLD_READABLE, MODE_WORLD_WRITEABLE
const CONTEXT_MODE_WORLD_READABLE: i32 = 0x0001;
const CONTEXT_MODE_WORLD_WRITEABLE: i32 = 0x0002;
// cite: frameworks-base Build.java#N
const ANDROID_SDK_N: i32 = 24;
// cite: frameworks-base Build.java#JELLY_BEAN_MR1
const ANDROID_SDK_JELLY_BEAN_MR1: i32 = 17;
// cite: frameworks-base PackageManager.java#GET_PROVIDERS
const PACKAGE_MANAGER_GET_PROVIDERS: i32 = 0x0000_0008;

impl DexProgram {
    pub(super) fn invoke_android_content_storage_builtin(
        &self,
        method: &DexMethodId,
        invocation: Option<&DexInvocation>,
    ) -> Option<DexExecutionResult> {
        if method.class_descriptor == "Landroid/content/ContentValues;"
            && method.name == "<init>"
            && method.proto.descriptor_is("()V")
        {
            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/content/ContentValues;"
            && method.name == "put"
            && method
                .proto
                .descriptor_is("(Ljava/lang/String;Ljava/lang/String;)V")
        {
            let invocation = invocation?;
            self.write_content_values_string(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/content/ContentValues;"
            && method.name == "put"
            && method
                .proto
                .descriptor_is("(Ljava/lang/String;Ljava/lang/Long;)V")
        {
            let invocation = invocation?;
            self.write_content_values_long(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/content/Intent;"
            && method.name == "getFlags"
            && method.proto.descriptor_is("()I")
        {
            return Some(DexExecutionResult::Int(0));
        }

        if method.class_descriptor == "Landroid/content/Intent;"
            && method.name == "<init>"
            && method.proto.descriptor_is("()V")
        {
            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/content/Intent;"
            && !self.guest_overrides(method, invocation)
        {
            if let Some(result) = self.intent_typed_extra(method, invocation?) {
                return Some(result);
            }
        }

        if method.matches_builtin_signature(
            "Landroid/content/Intent;",
            "<init>",
            "(Landroid/content/Intent;)V",
        ) {
            let invocation = invocation?;
            self.initialize_intent_copy(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/content/Intent;"
            && method.name == "putIntegerArrayListExtra"
            && method.proto.descriptor()
                == "(Ljava/lang/String;Ljava/util/ArrayList;)Landroid/content/Intent;"
        {
            let invocation = invocation?;
            let intent = self.put_intent_extra(invocation)?;

            return Some(DexExecutionResult::Value(intent));
        }

        if method.class_descriptor == "Landroid/content/Intent;"
            && method.name == "getIntegerArrayListExtra"
            && method
                .proto
                .descriptor_is("(Ljava/lang/String;)Ljava/util/ArrayList;")
        {
            let invocation = invocation?;
            let value = self.get_intent_extra(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Landroid/content/Intent;"
            && method.name == "<init>"
            && method
                .proto
                .descriptor_is("(Landroid/content/Context;Ljava/lang/Class;)V")
        {
            let invocation = invocation?;
            self.initialize_intent_for_class(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/content/Intent;"
            && method.name == "<init>"
            && method.proto.descriptor_is("(Ljava/lang/String;)V")
        {
            let invocation = invocation?;
            self.initialize_intent_for_action(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/content/Intent;"
            && method.name == "<init>"
            && method
                .proto
                .descriptor_is("(Ljava/lang/String;Landroid/net/Uri;)V")
        {
            let invocation = invocation?;
            self.initialize_intent_for_action_and_uri(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/content/Intent;"
            && method.name == "getData"
            && method.proto.descriptor_is("()Landroid/net/Uri;")
        {
            let invocation = invocation?;
            let value = self.intent_data(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Landroid/content/Intent;"
            && method.name == "setData"
            && method
                .proto
                .descriptor_is("(Landroid/net/Uri;)Landroid/content/Intent;")
        {
            let invocation = invocation?;
            let value = self.set_intent_data(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        // cite: frameworks-base Intent.java#setAction
        if method.class_descriptor == "Landroid/content/Intent;"
            && method.name == "setAction"
            && method
                .proto
                .descriptor_is("(Ljava/lang/String;)Landroid/content/Intent;")
        {
            let invocation = invocation?;
            let value = self.set_intent_action(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Landroid/content/Intent;"
            && method.name == "getAction"
            && method.proto.descriptor_is("()Ljava/lang/String;")
        {
            let invocation = invocation?;
            let value = self.intent_action(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        // cite: frameworks-base Intent.java#setDataAndType
        if method.class_descriptor == "Landroid/content/Intent;"
            && method.name == "setDataAndType"
            && method.proto.descriptor()
                == "(Landroid/net/Uri;Ljava/lang/String;)Landroid/content/Intent;"
        {
            let invocation = invocation?;
            let value = self.set_intent_data_and_type(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        // cite: frameworks-base Intent.java#getType, mType
        if method.class_descriptor == "Landroid/content/Intent;"
            && method.name == "getType"
            && method.proto.descriptor_is("()Ljava/lang/String;")
        {
            let invocation = invocation?;
            if !dex_value_is_object_like(invocation.arguments.first()?) {
                return None;
            }

            return Some(DexExecutionResult::Value(
                self.read_stored_instance_field(
                    &invocation.arguments[0],
                    &DexFieldId {
                        class_descriptor: "Landroid/content/Intent;".to_owned(),
                        type_descriptor: "Ljava/lang/String;".to_owned(),
                        name: "mType".to_owned(),
                    },
                )
                .unwrap_or(DexValue::Null),
            ));
        }

        if method.class_descriptor == "Landroid/content/Intent;"
            && method.name == "getType"
            && method.proto.descriptor_is("()Ljava/lang/String;")
        {
            let invocation = invocation?;
            let value = self.intent_type(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Landroid/content/Intent;"
            && method.name == "getExtras"
            && method.proto.descriptor_is("()Landroid/os/Bundle;")
        {
            return Some(DexExecutionResult::Value(DexValue::Null));
        }

        if method.class_descriptor == "Landroid/net/Uri;"
            && method.name == "parse"
            && method
                .proto
                .descriptor_is("(Ljava/lang/String;)Landroid/net/Uri;")
        {
            let invocation = invocation?;
            let value = self.parse_uri(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        // cite: frameworks-base Uri.java#getScheme, getQuery, getFragment, getHost, getPath
        if method.class_descriptor == "Landroid/net/Uri;"
            && method.proto.descriptor_is("()Ljava/lang/String;")
            && matches!(
                method.name.as_str(),
                "getScheme"
                    | "getQuery"
                    | "getFragment"
                    | "getHost"
                    | "getAuthority"
                    | "getPath"
                    | "getLastPathSegment"
            )
        {
            let invocation = invocation?;
            let component = match method.name.as_str() {
                "getScheme" => "scheme",
                "getQuery" => "query",
                "getFragment" => "fragment",
                "getHost" => "host",
                "getAuthority" => "authority",
                "getPath" => "path",
                "getLastPathSegment" => "lastPathSegment",
                _ => return None,
            };
            let value = self.uri_component(invocation, component)?;

            return Some(DexExecutionResult::Value(value));
        }

        // cite: frameworks-base Uri.java#getPathSegments
        if method.class_descriptor == "Landroid/net/Uri;"
            && method.name == "getPathSegments"
            && method.proto.descriptor_is("()Ljava/util/List;")
        {
            let invocation = invocation?;
            let value = self.uri_path_segments(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        // cite: frameworks-base Uri.java#getQueryParameterNames
        if method.class_descriptor == "Landroid/net/Uri;"
            && method.name == "getQueryParameterNames"
            && method.proto.descriptor_is("()Ljava/util/Set;")
        {
            let invocation = invocation?;
            let pairs = self.uri_query_pairs(invocation)?;
            let mut names = Vec::new();
            for (name, _) in pairs {
                let value = DexValue::String(std::rc::Rc::from(name));
                if !names.contains(&value) {
                    names.push(value);
                }
            }
            let set = self.allocate_object("Ljava/util/LinkedHashSet;".to_owned());
            self.runtime
                .borrow_mut()
                .list_entries
                .insert(map_identity_key(&set), names);

            return Some(DexExecutionResult::Value(set));
        }

        // cite: frameworks-base Uri.java#getQueryParameters, getQueryParameter
        if method.class_descriptor == "Landroid/net/Uri;"
            && method.name == "getQueryParameters"
            && method
                .proto
                .descriptor_is("(Ljava/lang/String;)Ljava/util/List;")
        {
            let invocation = invocation?;
            let DexValue::String(wanted) = invocation.arguments.get(1)? else {
                return None;
            };
            let pairs = self.uri_query_pairs(invocation)?;
            let values: Vec<DexValue> = pairs
                .into_iter()
                .filter(|(name, _)| name.as_str() == wanted.as_ref())
                .map(|(_, value)| DexValue::String(std::rc::Rc::from(value)))
                .collect();
            let list = self.allocate_object("Ljava/util/ArrayList;".to_owned());
            self.runtime
                .borrow_mut()
                .list_entries
                .insert(map_identity_key(&list), values);

            return Some(DexExecutionResult::Value(list));
        }

        if method.class_descriptor == "Landroid/net/Uri;"
            && method.name == "getQueryParameter"
            && method
                .proto
                .descriptor_is("(Ljava/lang/String;)Ljava/lang/String;")
        {
            let invocation = invocation?;
            let DexValue::String(wanted) = invocation.arguments.get(1)? else {
                return None;
            };
            let pairs = self.uri_query_pairs(invocation)?;
            let value = pairs
                .into_iter()
                .find(|(name, _)| name.as_str() == wanted.as_ref())
                .map(|(_, value)| DexValue::String(std::rc::Rc::from(value)))
                .unwrap_or(DexValue::Null);

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Landroid/net/Uri;"
            && method.name == "toString"
            && method.proto.descriptor_is("()Ljava/lang/String;")
        {
            let invocation = invocation?;
            let value = self.uri_to_string(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Landroid/content/IntentFilter;"
            && method.name == "<init>"
            && method.proto.descriptor_is("()V")
        {
            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/content/IntentFilter;"
            && matches!(method.name.as_str(), "addAction" | "<init>")
            && method.proto.descriptor_is("(Ljava/lang/String;)V")
        {
            // cite: frameworks-base IntentFilter.java#IntentFilter, addAction
            let invocation = invocation?;
            self.add_intent_filter_action(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/content/IntentFilter;"
            && matches!(
                (method.name.as_str(), method.proto.descriptor().as_str()),
                ("getAction", "(I)Ljava/lang/String;")
                    | ("hasAction" | "matchAction", "(Ljava/lang/String;)Z")
            )
            && !self.guest_overrides(method, invocation)
        {
            // cite: frameworks-base IntentFilter.java#getAction, hasAction, matchAction
            let invocation = invocation?;
            let filter = invocation.arguments.first()?;
            let actions = self
                .runtime
                .borrow()
                .intent_filter_actions
                .get(&map_identity_key(filter))
                .cloned()
                .unwrap_or_default();
            return Some(match invocation.arguments.get(1)? {
                DexValue::Int(index) => match usize::try_from(*index)
                    .ok()
                    .and_then(|index| actions.get(index))
                {
                    Some(action) => DexExecutionResult::Value(DexValue::String(std::rc::Rc::from(
                        action.as_str(),
                    ))),
                    None => DexExecutionResult::Throw(
                        self.allocate_object("Ljava/lang/IndexOutOfBoundsException;".to_owned()),
                    ),
                },
                DexValue::String(action) => DexExecutionResult::Int(i32::from(
                    actions
                        .iter()
                        .any(|known| known.as_str() == action.as_ref()),
                )),
                DexValue::Null if method.name != "getAction" => DexExecutionResult::Int(0),
                _ => return None,
            });
        }

        if method.class_descriptor == "Landroid/content/IntentFilter;"
            && method.name == "countActions"
            && method.proto.descriptor_is("()I")
        {
            let invocation = invocation?;
            let value = self.intent_filter_action_count(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Landroid/content/BroadcastReceiver;"
            && method.name == "<init>"
            && method.proto.descriptor_is("()V")
        {
            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/content/ComponentName;"
            && !self.guest_overrides(method, invocation)
        {
            if let Some(result) = self.component_name_method(method, invocation?) {
                return Some(result);
            }
        }

        if method.class_descriptor == "Landroid/content/Context;"
            && method.name == "getApplicationContext"
            && method.proto.descriptor_is("()Landroid/content/Context;")
        {
            return Some(DexExecutionResult::Value(self.application_value()));
        }

        if method.class_descriptor == "Landroid/content/Context;"
            && method.name == "getCacheDir"
            && method.proto.descriptor_is("()Ljava/io/File;")
        {
            let invocation = invocation?;
            let value = self.context_private_directory_file(invocation, "cache")?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Landroid/content/Context;"
            && method.name == "getDir"
            && method
                .proto
                .descriptor_is("(Ljava/lang/String;I)Ljava/io/File;")
        {
            let invocation = invocation?;

            return self.context_app_dir_file(invocation);
        }

        if method.class_descriptor == "Landroid/content/Context;"
            && method.name == "getFilesDir"
            && method.proto.descriptor_is("()Ljava/io/File;")
        {
            let invocation = invocation?;
            let value = self.context_private_directory_file(invocation, "files")?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Landroid/content/Context;"
            && method.name == "getExternalFilesDir"
            && method
                .proto
                .descriptor_is("(Ljava/lang/String;)Ljava/io/File;")
        {
            let invocation = invocation?;
            let value = self.context_external_files_dir(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        // cite: frameworks-base ContextImpl.java#getExternalCacheDir, getExternalCacheDirs
        if method.matches_builtin_signature(
            "Landroid/content/Context;",
            "getExternalCacheDir",
            "()Ljava/io/File;",
        ) {
            context_receiver(invocation?)?;
            let value = self.build_external_app_dir("cache", None)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Landroid/content/Context;"
            && method.name == "getExternalFilesDirs"
            && method
                .proto
                .descriptor_is("(Ljava/lang/String;)[Ljava/io/File;")
        {
            let invocation = invocation?;
            let value = self.context_external_files_dirs(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Landroid/content/Context;"
            && method.name == "getDataDir"
            && method.proto.descriptor_is("()Ljava/io/File;")
        {
            let invocation = invocation?;
            let value = self.context_data_dir_file(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Landroid/content/Context;"
            && method.name == "getNoBackupFilesDir"
            && method.proto.descriptor_is("()Ljava/io/File;")
        {
            let invocation = invocation?;
            let value = self.context_private_directory_file(invocation, "no_backup")?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Landroid/content/Context;"
            && method.name == "getCodeCacheDir"
            && method.proto.descriptor_is("()Ljava/io/File;")
        {
            let invocation = invocation?;
            let value = self.context_private_directory_file(invocation, "code_cache")?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Landroid/content/Context;"
            && method.name == "getSharedPreferencesPath"
            && method
                .proto
                .descriptor_is("(Ljava/lang/String;)Ljava/io/File;")
        {
            let invocation = invocation?;
            return self.context_shared_preferences_path_file(invocation);
        }

        if method.class_descriptor == "Landroid/content/Context;"
            && method.name == "createDeviceProtectedStorageContext"
            && method.proto.descriptor_is("()Landroid/content/Context;")
        {
            let invocation = invocation?;
            let context = context_receiver(invocation)?;
            let value = self.create_private_storage_context(
                context,
                crate::guest_filesystem::PrivateStorageKind::DeviceProtected,
            )?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Landroid/content/Context;"
            && method.name == "createCredentialProtectedStorageContext"
            && method.proto.descriptor_is("()Landroid/content/Context;")
        {
            let invocation = invocation?;
            let context = context_receiver(invocation)?;
            let value = self.create_private_storage_context(
                context,
                crate::guest_filesystem::PrivateStorageKind::CredentialProtected,
            )?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Landroid/content/Context;"
            && method.name == "isDeviceProtectedStorage"
            && method.proto.descriptor_is("()Z")
        {
            let invocation = invocation?;
            let context = context_receiver(invocation)?;
            let value = self.context_is_private_storage(
                context,
                crate::guest_filesystem::PrivateStorageKind::DeviceProtected,
            )?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Landroid/content/Context;"
            && method.name == "isCredentialProtectedStorage"
            && method.proto.descriptor_is("()Z")
        {
            let invocation = invocation?;
            let context = context_receiver(invocation)?;
            let value = self.context_is_private_storage(
                context,
                crate::guest_filesystem::PrivateStorageKind::CredentialProtected,
            )?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Landroid/content/Context;"
            && method.name == "getDatabasePath"
            && method
                .proto
                .descriptor_is("(Ljava/lang/String;)Ljava/io/File;")
        {
            let invocation = invocation?;
            return self.context_database_path_file(invocation);
        }

        if method.class_descriptor == "Landroid/content/Context;"
            && method.name == "registerComponentCallbacks"
            && method
                .proto
                .descriptor_is("(Landroid/content/ComponentCallbacks;)V")
        {
            let invocation = invocation?;
            self.register_component_callback(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/content/Context;"
            && method.name == "unregisterComponentCallbacks"
            && method
                .proto
                .descriptor_is("(Landroid/content/ComponentCallbacks;)V")
        {
            let invocation = invocation?;
            self.unregister_component_callback(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/content/Context;"
            && method.name == "unregisterReceiver"
            && method
                .proto
                .descriptor_is("(Landroid/content/BroadcastReceiver;)V")
        {
            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/content/Context;"
            && method.name == "registerReceiver"
            && matches!(
                method.proto.descriptor().as_str(),
                "(Landroid/content/BroadcastReceiver;Landroid/content/IntentFilter;)Landroid/content/Intent;"
                    | "(Landroid/content/BroadcastReceiver;Landroid/content/IntentFilter;I)Landroid/content/Intent;"
            )
        {
            // cite: frameworks-base ContextImpl.java#registerReceiverInternal
            // cite: frameworks-base BatteryService.java#sendBatteryChangedIntentLocked
            let invocation = invocation?;
            let filter = invocation.arguments.get(2)?;
            let wants_sticky_battery = self
                .runtime
                .borrow()
                .intent_filter_actions
                .get(&map_identity_key(filter))
                .is_some_and(|actions| {
                    actions
                        .iter()
                        .any(|action| action == "android.intent.action.BATTERY_CHANGED")
                });
            if wants_sticky_battery {
                return None;
            }
            return Some(DexExecutionResult::Value(DexValue::Null));
        }

        if method.class_descriptor == "Landroid/content/pm/PackageManager;"
            && method.name == "hasSystemFeature"
            && method.proto.descriptor_is("(Ljava/lang/String;)Z")
        {
            return Some(DexExecutionResult::Int(0));
        }

        if method.class_descriptor == "Landroid/content/pm/PackageManager;"
            && method.name == "checkPermission"
            && method
                .proto
                .descriptor_is("(Ljava/lang/String;Ljava/lang/String;)I")
        {
            let invocation = invocation?;
            let value = self.package_manager_check_permission(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Landroid/content/pm/PackageManager;"
            && method.name == "getApplicationInfo"
            && method
                .proto
                .descriptor_is("(Ljava/lang/String;I)Landroid/content/pm/ApplicationInfo;")
        {
            let invocation = invocation?;

            return self.package_manager_get_application_info(invocation);
        }

        if method.class_descriptor == "Landroid/content/pm/PackageManager;"
            && method.name == "getPackageInfo"
            && method
                .proto
                .descriptor_is("(Ljava/lang/String;I)Landroid/content/pm/PackageInfo;")
        {
            let invocation = invocation?;

            return self.package_manager_get_package_info(invocation);
        }

        if method.class_descriptor == "Landroid/content/pm/PackageManager;"
            && method.name == "getActivityInfo"
            && method.proto.descriptor()
                == "(Landroid/content/ComponentName;I)Landroid/content/pm/ActivityInfo;"
        {
            return Some(DexExecutionResult::Value(DexValue::Reference(
                std::rc::Rc::from("Landroid/content/pm/ActivityInfo;.bootstrap"),
            )));
        }

        if method.class_descriptor == "Landroid/content/SharedPreferences;"
            && method.name == "contains"
            && method.proto.descriptor_is("(Ljava/lang/String;)Z")
        {
            let invocation = invocation?;
            let value = self.shared_preferences_contains(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Landroid/content/SharedPreferences;"
            && method.name == "getBoolean"
            && method.proto.descriptor_is("(Ljava/lang/String;Z)Z")
        {
            let invocation = invocation?;
            let default_value = self.shared_preferences_get_boolean(invocation)?;

            return Some(DexExecutionResult::Int(default_value));
        }

        if method.class_descriptor == "Landroid/content/SharedPreferences;"
            && method.name == "getString"
            && method.proto.descriptor()
                == "(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;"
        {
            let invocation = invocation?;
            let default_value = self.shared_preferences_get_string(invocation)?;

            return Some(DexExecutionResult::Value(default_value));
        }

        if method.class_descriptor == "Landroid/content/SharedPreferences;"
            && method.name == "getStringSet"
            && method.proto.descriptor() == "(Ljava/lang/String;Ljava/util/Set;)Ljava/util/Set;"
        {
            // cite: frameworks-base SharedPreferencesImpl.java#getStringSet, awaitLoadedLocked
            let invocation = invocation?;
            let value = self.shared_preferences_get_string_set(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Landroid/content/SharedPreferences;"
            && method.name == "getInt"
            && method.proto.descriptor_is("(Ljava/lang/String;I)I")
        {
            let invocation = invocation?;
            let default_value = self.shared_preferences_get_int(invocation)?;

            return Some(DexExecutionResult::Int(default_value));
        }

        if method.class_descriptor == "Landroid/content/SharedPreferences;"
            && method.name == "getLong"
            && method.proto.descriptor_is("(Ljava/lang/String;J)J")
        {
            let invocation = invocation?;
            let default_value = self.shared_preferences_get_long(invocation)?;

            return Some(DexExecutionResult::Value(DexValue::Long(default_value)));
        }

        if method.class_descriptor == "Landroid/content/SharedPreferences;"
            && method.name == "edit"
            && method
                .proto
                .descriptor_is("()Landroid/content/SharedPreferences$Editor;")
        {
            let invocation = invocation?;
            let editor = self.shared_preferences_edit(invocation)?;

            return Some(DexExecutionResult::Value(editor));
        }

        if method.class_descriptor == "Landroid/content/SharedPreferences$Editor;"
            && method.name == "putBoolean"
            && method.proto.descriptor()
                == "(Ljava/lang/String;Z)Landroid/content/SharedPreferences$Editor;"
        {
            let invocation = invocation?;
            let editor = self.shared_preferences_editor_put_boolean(invocation)?;

            return Some(DexExecutionResult::Value(editor));
        }

        if method.class_descriptor == "Landroid/content/SharedPreferences$Editor;"
            && method.name == "putString"
            && method.proto.descriptor()
                == "(Ljava/lang/String;Ljava/lang/String;)Landroid/content/SharedPreferences$Editor;"
        {
            let invocation = invocation?;
            let editor = self.shared_preferences_editor_put_string(invocation)?;

            return Some(DexExecutionResult::Value(editor));
        }

        if method.class_descriptor == "Landroid/content/SharedPreferences$Editor;"
            && method.name == "putInt"
            && method.proto.descriptor()
                == "(Ljava/lang/String;I)Landroid/content/SharedPreferences$Editor;"
        {
            let invocation = invocation?;
            let editor = self.shared_preferences_editor_put_int(invocation)?;

            return Some(DexExecutionResult::Value(editor));
        }

        if method.class_descriptor == "Landroid/content/SharedPreferences$Editor;"
            && method.name == "putLong"
            && method.proto.descriptor()
                == "(Ljava/lang/String;J)Landroid/content/SharedPreferences$Editor;"
        {
            let invocation = invocation?;
            let editor = self.shared_preferences_editor_put_long(invocation)?;

            return Some(DexExecutionResult::Value(editor));
        }

        if method.class_descriptor == "Landroid/content/SharedPreferences$Editor;"
            && method.name == "remove"
            && method.proto.descriptor()
                == "(Ljava/lang/String;)Landroid/content/SharedPreferences$Editor;"
        {
            let invocation = invocation?;
            let editor = self.shared_preferences_editor_remove(invocation)?;

            return Some(DexExecutionResult::Value(editor));
        }

        if method.class_descriptor == "Landroid/content/SharedPreferences$Editor;"
            && method.name == "clear"
            && method
                .proto
                .descriptor_is("()Landroid/content/SharedPreferences$Editor;")
        {
            let invocation = invocation?;
            let editor = self.shared_preferences_editor_clear(invocation)?;

            return Some(DexExecutionResult::Value(editor));
        }

        if method.class_descriptor == "Landroid/content/SharedPreferences$Editor;"
            && method.name == "apply"
            && method.proto.descriptor_is("()V")
        {
            let invocation = invocation?;
            self.shared_preferences_editor_apply(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/content/SharedPreferences$Editor;"
            && method.name == "commit"
            && method.proto.descriptor_is("()Z")
        {
            let invocation = invocation?;
            let value = self.shared_preferences_editor_commit(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        None
    }

    fn context_data_dir_file(&self, invocation: &DexInvocation) -> Option<DexValue> {
        // cite: frameworks-base ContextImpl.java#getDataDir
        let context = context_receiver(invocation)?;
        let storage = self.context_private_storage_kind(context)?;
        let path = self
            .guest_filesystem
            .as_ref()?
            .private_guest_path(storage, "");
        Some(DexValue::Reference(std::rc::Rc::from(format!(
            "Ljava/io/File;:{path}"
        ))))
    }

    fn context_private_directory_file(
        &self,
        invocation: &DexInvocation,
        child: &str,
    ) -> Option<DexValue> {
        // cite: frameworks-base ContextImpl.java#getFilesDir, getNoBackupFilesDir, getCacheDir, getCodeCacheDir, getDataDir
        let context = context_receiver(invocation)?;
        let path = self.ensure_context_private_directory(context, child)?;
        Some(DexValue::Reference(std::rc::Rc::from(format!(
            "Ljava/io/File;:{path}"
        ))))
    }

    // cite: frameworks-base ContextImpl.java#getExternalFilesDirs
    // cite: frameworks-base Environment.java#buildExternalStorageAppFilesDirs, buildPaths
    fn context_external_files_dirs(&self, invocation: &DexInvocation) -> Option<DexValue> {
        let _context = context_receiver(invocation)?;
        match &invocation.arguments[1] {
            DexValue::String(type_name) => {
                self.build_external_files_dirs_array(Some(type_name.as_ref()))
            }
            DexValue::Null | DexValue::Int(0) => self.build_external_files_dirs_array(None),
            _ => None,
        }
    }

    fn build_external_files_dirs_array(&self, type_name: Option<&str>) -> Option<DexValue> {
        let file = self.build_external_files_dir(type_name)?;

        Some(self.allocate_filled_array("[Ljava/io/File;".to_owned(), vec![file]))
    }

    // cite: frameworks-base ContextImpl.java#getExternalFilesDir, getExternalFilesDirs
    fn context_external_files_dir(&self, invocation: &DexInvocation) -> Option<DexValue> {
        let _context = context_receiver(invocation)?;
        match &invocation.arguments[1] {
            DexValue::String(type_name) => self.build_external_files_dir(Some(type_name.as_ref())),
            DexValue::Null | DexValue::Int(0) => self.build_external_files_dir(None),
            _ => None,
        }
    }

    fn build_external_files_dir(&self, type_name: Option<&str>) -> Option<DexValue> {
        self.build_external_app_dir("files", type_name)
    }

    // cite: frameworks-base ContextImpl.java#getExternalFilesDirs, getExternalCacheDirs, ensureExternalDirsExistOrFilter
    // cite: frameworks-base Environment.java#buildExternalStorageAppFilesDirs, buildExternalStorageAppCacheDirs
    fn build_external_app_dir(&self, leaf: &str, type_name: Option<&str>) -> Option<DexValue> {
        let package_name = self.package_name.as_deref()?;
        let mut guest_path = format!(
            "{}/Android/data/{package_name}/{leaf}",
            crate::external_storage::GUEST_PRIMARY_EXTERNAL_ROOT
        );
        if let Some(type_name) = type_name {
            guest_path.push('/');
            guest_path.push_str(type_name);
        }

        if let Some(filesystem) = self.guest_filesystem.as_ref() {
            if let Ok(resolved) = filesystem.resolve(
                &guest_path,
                crate::external_storage::ExternalStorageAccess::Write,
                &self.external_storage_policy,
            ) {
                let _ = resolved.mkdir_all();
            }
        }

        Some(DexValue::Reference(std::rc::Rc::from(format!(
            "Ljava/io/File;:{guest_path}"
        ))))
    }

    fn context_database_path_file(&self, invocation: &DexInvocation) -> Option<DexExecutionResult> {
        if invocation.arguments.len() != 2 {
            return None;
        }
        let context = context_receiver(invocation)?;
        let storage = self.context_private_storage_kind(context)?;

        let DexValue::String(name) = &invocation.arguments[1] else {
            return None;
        };
        if name.is_empty() {
            return Some(DexExecutionResult::Throw(self.allocate_object(
                "Ljava/lang/StringIndexOutOfBoundsException;".to_owned(),
            )));
        }
        let filesystem = self.guest_filesystem.as_ref()?;

        // cite: frameworks-base ContextImpl.java#getDatabasePath, getDatabasesDir, makeFilename
        let path = if name.starts_with('/') {
            let path = crate::guest_filesystem::normalize_guest_file_path(name);
            self.try_create_absolute_database_parent(&path);
            path
        } else {
            if name.contains('/') {
                return Some(DexExecutionResult::Throw(
                    self.allocate_object("Ljava/lang/IllegalArgumentException;".to_owned()),
                ));
            }
            let database_dir = filesystem.private_guest_path(storage, "databases");
            if let Ok(directory) = filesystem.resolve(
                &database_dir,
                crate::external_storage::ExternalStorageAccess::Write,
                &self.external_storage_policy,
            ) {
                let _ = directory.mkdir_all();
            }
            format!("{database_dir}/{name}")
        };

        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from(format!("Ljava/io/File;:{path}")),
        )))
    }

    fn package_manager_get_application_info(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexExecutionResult> {
        // cite: frameworks-base ApplicationPackageManager.java#getApplicationInfo, getApplicationInfoAsUser
        // cite: frameworks-base PackageManager.java#getApplicationInfo
        let [_receiver, package_name, DexValue::Int(_flags)] = invocation.arguments.as_slice()
        else {
            return None;
        };
        let installed = match package_name {
            DexValue::String(name) => name.as_ref() == self.bootstrap_process_name(),
            DexValue::Null => false,
            _ => return None,
        };
        if installed {
            return Some(DexExecutionResult::Value(DexValue::Reference(
                std::rc::Rc::from("Landroid/content/pm/ApplicationInfo;.bootstrap"),
            )));
        }

        let detail_message = match package_name {
            DexValue::String(name) => Some(name.to_string()),
            _ => None,
        };

        Some(DexExecutionResult::Throw(
            self.allocate_generated_exception(DexGeneratedException {
                class_descriptor: "Landroid/content/pm/PackageManager$NameNotFoundException;"
                    .to_owned(),
                detail_message,
            }),
        ))
    }

    fn package_manager_get_package_info(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexExecutionResult> {
        // cite: frameworks-base ApplicationPackageManager.java#getPackageInfo, getPackageInfoAsUser
        // cite: frameworks-base PackageParser.java#generatePackageInfo
        let [_receiver, package_name, DexValue::Int(flags)] = invocation.arguments.as_slice()
        else {
            return None;
        };
        let installed = match package_name {
            DexValue::String(name) => name.as_ref() == self.bootstrap_process_name(),
            DexValue::Null => false,
            _ => return None,
        };
        if !installed {
            let detail_message = match package_name {
                DexValue::String(name) => Some(name.to_string()),
                _ => None,
            };

            return Some(DexExecutionResult::Throw(
                self.allocate_generated_exception(DexGeneratedException {
                    class_descriptor: "Landroid/content/pm/PackageManager$NameNotFoundException;"
                        .to_owned(),
                    detail_message,
                }),
            ));
        }
        if flags & !PACKAGE_MANAGER_GET_PROVIDERS != 0 {
            return None;
        }
        let version_code = match self.manifest_version_code.as_deref() {
            Some(text) => text.parse::<i32>().ok()?,
            None => 0,
        };

        let application_info = DexValue::Reference(std::rc::Rc::from(
            "Landroid/content/pm/ApplicationInfo;.bootstrap",
        ));
        let package_info = self.allocate_object("Landroid/content/pm/PackageInfo;".to_owned());
        let package_info_field = |name: &str, type_descriptor: &str| DexFieldId {
            class_descriptor: "Landroid/content/pm/PackageInfo;".to_owned(),
            type_descriptor: type_descriptor.to_owned(),
            name: name.to_owned(),
        };
        let package_name_text = package_name_text(package_name)?;
        let fields = [
            (
                package_info_field("packageName", "Ljava/lang/String;"),
                DexValue::String(std::rc::Rc::from(package_name_text.as_str())),
            ),
            (
                package_info_field("versionCode", "I"),
                DexValue::Int(version_code),
            ),
            (
                package_info_field("versionName", "Ljava/lang/String;"),
                match &self.manifest_version_name {
                    Some(name) => DexValue::String(std::rc::Rc::from(name.as_str())),
                    None => DexValue::Null,
                },
            ),
            (
                package_info_field("applicationInfo", "Landroid/content/pm/ApplicationInfo;"),
                application_info.clone(),
            ),
        ];
        for (field, value) in fields {
            self.write_stored_instance_field(&package_info, &field, value);
        }

        if flags & PACKAGE_MANAGER_GET_PROVIDERS != 0 {
            let providers = self.package_info_providers(&package_name_text, &application_info)?;
            self.write_stored_instance_field(
                &package_info,
                &package_info_field("providers", "[Landroid/content/pm/ProviderInfo;"),
                providers,
            );
        }

        Some(DexExecutionResult::Value(package_info))
    }

    fn package_info_providers(
        &self,
        package_name: &str,
        application_info: &DexValue,
    ) -> Option<DexValue> {
        // cite: frameworks-base PackageParser.java#generatePackageInfo, generateProviderInfo, parseProvider
        let enabled = self
            .manifest_providers
            .iter()
            .filter(|provider| provider.is_enabled())
            .collect::<Vec<_>>();
        if enabled.is_empty() {
            return Some(DexValue::Null);
        }
        let array = self.allocate_array(
            "[Landroid/content/pm/ProviderInfo;".to_owned(),
            i32::try_from(enabled.len()).ok()?,
        )?;
        for (index, provider) in enabled.into_iter().enumerate() {
            let info = self.provider_info_value(package_name, application_info, provider);
            self.write_array_value(&array, i32::try_from(index).ok()?, info)?;
        }

        Some(array)
    }

    pub(in crate::dex_run) fn provider_info_value(
        &self,
        package_name: &str,
        application_info: &DexValue,
        provider: &manifest::Provider,
    ) -> DexValue {
        // cite: frameworks-base PackageParser.java#generateProviderInfo
        let exported_default = self.target_sdk_version < ANDROID_SDK_JELLY_BEAN_MR1;
        let info = self.allocate_object("Landroid/content/pm/ProviderInfo;".to_owned());
        let info_field = |name: &str, type_descriptor: &str| DexFieldId {
            class_descriptor: "Landroid/content/pm/ProviderInfo;".to_owned(),
            type_descriptor: type_descriptor.to_owned(),
            name: name.to_owned(),
        };
        let fields = [
            (
                info_field("name", "Ljava/lang/String;"),
                DexValue::String(std::rc::Rc::from(provider.name.as_str())),
            ),
            (
                info_field("packageName", "Ljava/lang/String;"),
                DexValue::String(std::rc::Rc::from(package_name)),
            ),
            (
                info_field("authority", "Ljava/lang/String;"),
                DexValue::String(std::rc::Rc::from(provider.authority.as_str())),
            ),
            (
                info_field("exported", "Z"),
                DexValue::Int(i32::from(provider.exported.unwrap_or(exported_default))),
            ),
            (info_field("enabled", "Z"), DexValue::Int(1)),
            (
                info_field("applicationInfo", "Landroid/content/pm/ApplicationInfo;"),
                application_info.clone(),
            ),
        ];
        for (field, value) in fields {
            self.write_stored_instance_field(&info, &field, value);
        }
        info
    }

    fn context_app_dir_file(&self, invocation: &DexInvocation) -> Option<DexExecutionResult> {
        // cite: frameworks-base ContextImpl.java#getDir, checkMode, makeFilename
        let [context, name, DexValue::Int(mode)] = invocation.arguments.as_slice() else {
            return None;
        };
        if !dex_value_is_object_like(context) {
            return None;
        }
        self.context_private_storage_kind(context)?;
        if self.target_sdk_version >= ANDROID_SDK_N {
            for (flag, message) in [
                (
                    CONTEXT_MODE_WORLD_READABLE,
                    "MODE_WORLD_READABLE no longer supported",
                ),
                (
                    CONTEXT_MODE_WORLD_WRITEABLE,
                    "MODE_WORLD_WRITEABLE no longer supported",
                ),
            ] {
                if mode & flag != 0 {
                    return Some(DexExecutionResult::Throw(
                        self.allocate_generated_exception(DexGeneratedException {
                            class_descriptor: "Ljava/lang/SecurityException;".to_owned(),
                            detail_message: Some(message.to_owned()),
                        }),
                    ));
                }
            }
        }
        let name = match name {
            DexValue::String(name) => name.as_ref(),
            DexValue::Null => "null",
            _ => return None,
        };
        let prefixed = format!("app_{name}");
        if prefixed.contains('/') {
            return Some(DexExecutionResult::Throw(
                self.allocate_generated_exception(DexGeneratedException {
                    class_descriptor: "Ljava/lang/IllegalArgumentException;".to_owned(),
                    detail_message: Some(format!("File {prefixed} contains a path separator")),
                }),
            ));
        }
        let path = self.ensure_context_private_directory(context, &prefixed)?;

        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from(format!("Ljava/io/File;:{path}")),
        )))
    }

    fn package_manager_check_permission(&self, invocation: &DexInvocation) -> Option<i32> {
        if invocation.arguments.len() != 3 {
            return None;
        }
        let DexValue::String(permission) = &invocation.arguments[1] else {
            return None;
        };
        let DexValue::String(package_name) = &invocation.arguments[2] else {
            return None;
        };

        // cite: frameworks-base ApplicationPackageManager.java#checkPermission
        // cite: frameworks-base PermissionManager.java#checkPackageNamePermission
        if package_name.as_ref() != self.bootstrap_process_name() {
            return Some(ANDROID_PERMISSION_DENIED);
        }

        let granted = self
            .external_storage_policy
            .permission_granted(permission)
            .unwrap_or(true);

        Some(if granted {
            ANDROID_PERMISSION_GRANTED
        } else {
            ANDROID_PERMISSION_DENIED
        })
    }

    fn context_shared_preferences_path_file(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexExecutionResult> {
        if invocation.arguments.len() != 2 {
            return None;
        }
        let context = context_receiver(invocation)?;
        let DexValue::String(name) = &invocation.arguments[1] else {
            return None;
        };
        if name.contains('/') {
            return Some(DexExecutionResult::Throw(
                self.allocate_object("Ljava/lang/IllegalArgumentException;".to_owned()),
            ));
        }
        let directory = self.ensure_context_private_directory(context, "shared_prefs")?;
        let path = format!("{directory}/{name}.xml");

        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from(format!("Ljava/io/File;:{path}")),
        )))
    }

    fn ensure_context_private_directory(&self, context: &DexValue, child: &str) -> Option<String> {
        let filesystem = self.guest_filesystem.as_ref()?;
        let storage = self.context_private_storage_kind(context)?;
        let guest_path = filesystem.private_guest_path(storage, child);
        if let Ok(path) = filesystem.resolve(
            &guest_path,
            crate::external_storage::ExternalStorageAccess::Write,
            &self.external_storage_policy,
        ) {
            let _ = path.mkdir_all();
        }

        Some(guest_path)
    }

    fn try_create_absolute_database_parent(&self, guest_path: &str) {
        let Some((parent, _)) = guest_path.rsplit_once('/') else {
            return;
        };
        let parent = if parent.is_empty() { "/" } else { parent };
        let Some(filesystem) = self.guest_filesystem.as_ref() else {
            return;
        };
        let Ok(parent) = filesystem.resolve(
            parent,
            crate::external_storage::ExternalStorageAccess::Write,
            &self.external_storage_policy,
        ) else {
            return;
        };
        if !parent
            .metadata()
            .is_ok_and(|metadata| metadata.kind == crate::capability_fs::MetadataKind::Directory)
        {
            let _ = parent.mkdir();
        }
    }
}

fn context_receiver(invocation: &DexInvocation) -> Option<&DexValue> {
    if invocation.arguments.len() != 1 && invocation.arguments.len() != 2 {
        return None;
    }
    let context = invocation.arguments.first()?;
    dex_value_is_object_like(context).then_some(context)
}

fn package_name_text(value: &DexValue) -> Option<String> {
    match value {
        DexValue::String(name) => Some(name.to_string()),
        _ => None,
    }
}

impl DexProgram {
    // cite: frameworks-base Intent.java#Intent
    fn initialize_intent_copy(&self, invocation: &DexInvocation) -> Option<()> {
        let [intent, source] = invocation.arguments.as_slice() else {
            return None;
        };
        if !dex_value_is_object_like(intent) || !dex_value_is_object_like(source) {
            return None;
        }
        let stored_type = DexFieldId {
            class_descriptor: "Landroid/content/Intent;".to_owned(),
            type_descriptor: "Ljava/lang/String;".to_owned(),
            name: "mType".to_owned(),
        };
        for field in [
            intent_target_class_field(),
            intent_action_field(),
            intent_type_field(),
            intent_data_uri_field(),
            stored_type,
        ] {
            if let Some(value) = self.read_stored_instance_field(source, &field) {
                self.write_stored_instance_field(intent, &field, value);
            }
        }

        let (source_key, intent_key) = (map_identity_key(source), map_identity_key(intent));
        let mut runtime = self.runtime.borrow_mut();
        let extras: Vec<_> = runtime
            .intent_extras
            .iter()
            .filter(|((owner, _), _)| *owner == source_key)
            .map(|((_, name), value)| (name.clone(), value.clone()))
            .collect();
        for (name, value) in extras {
            if let Some(type_descriptor) = runtime
                .intent_extra_types
                .get(&(source_key.clone(), name.clone()))
                .cloned()
            {
                runtime
                    .intent_extra_types
                    .insert((intent_key.clone(), name.clone()), type_descriptor);
            }
            runtime
                .intent_extras
                .insert((intent_key.clone(), name), value);
        }

        Some(())
    }
}

const INTENT_EXTRA_TYPES: &[&str] = &["Ljava/lang/String;", "I", "J", "Z"];

impl DexProgram {
    // cite: frameworks-base Intent.java#putExtra, getStringExtra, getIntExtra, getLongExtra, getBooleanExtra, hasExtra
    // cite: frameworks-base BaseBundle.java#getString, getInt, getLong, getBoolean, containsKey
    fn intent_typed_extra(
        &self,
        method: &DexMethodId,
        invocation: &DexInvocation,
    ) -> Option<DexExecutionResult> {
        let arguments = invocation.arguments.as_slice();
        let intent = arguments.first()?;
        if !dex_value_is_object_like(intent) {
            return None;
        }
        let DexValue::String(name) = arguments.get(1)? else {
            return None;
        };
        let key = (map_identity_key(intent), name.to_string());
        let parameters = &method.proto.parameters;

        if method.name == "putExtra"
            && method.proto.return_type == "Landroid/content/Intent;"
            && parameters.len() == 2
            && INTENT_EXTRA_TYPES.contains(&parameters[1].as_str())
        {
            let mut runtime = self.runtime.borrow_mut();
            runtime
                .intent_extras
                .insert(key.clone(), arguments.get(2)?.clone());
            runtime
                .intent_extra_types
                .insert(key, parameters[1].clone());
            return Some(DexExecutionResult::Value(intent.clone()));
        }

        let runtime = self.runtime.borrow();
        let stored = runtime.intent_extras.get(&key);
        let typed = |type_descriptor: &str| {
            stored.filter(|_| {
                runtime
                    .intent_extra_types
                    .get(&key)
                    .is_none_or(|stored_type| stored_type == type_descriptor)
            })
        };
        Some(
            match (method.name.as_str(), method.proto.descriptor().as_str()) {
                ("hasExtra", "(Ljava/lang/String;)Z") => {
                    DexExecutionResult::Int(i32::from(stored.is_some()))
                }
                ("getStringExtra", "(Ljava/lang/String;)Ljava/lang/String;") => {
                    DexExecutionResult::Value(
                        typed("Ljava/lang/String;")
                            .filter(|value| matches!(value, DexValue::String(_)))
                            .cloned()
                            .unwrap_or(DexValue::Null),
                    )
                }
                ("getIntExtra", "(Ljava/lang/String;I)I")
                | ("getBooleanExtra", "(Ljava/lang/String;Z)Z") => {
                    let type_descriptor = if method.name == "getIntExtra" {
                        "I"
                    } else {
                        "Z"
                    };
                    match typed(type_descriptor) {
                        Some(DexValue::Int(value)) => DexExecutionResult::Int(*value),
                        _ => match arguments.get(2)? {
                            DexValue::Int(default) => DexExecutionResult::Int(*default),
                            _ => return None,
                        },
                    }
                }
                ("getLongExtra", "(Ljava/lang/String;J)J") => match typed("J") {
                    Some(DexValue::Long(value)) => {
                        DexExecutionResult::Value(DexValue::Long(*value))
                    }
                    _ => match arguments.get(2)? {
                        DexValue::Long(default) => {
                            DexExecutionResult::Value(DexValue::Long(*default))
                        }
                        _ => return None,
                    },
                },
                _ => return None,
            },
        )
    }
}

fn component_name_field(name: &str) -> DexFieldId {
    DexFieldId {
        class_descriptor: "Landroid/content/ComponentName;".to_owned(),
        type_descriptor: "Ljava/lang/String;".to_owned(),
        name: name.to_owned(),
    }
}

// cite: frameworks-base ComponentName.java#getShortClassName, appendShortClassName
fn component_short_class_name(package: &str, class: &str) -> String {
    match class.strip_prefix(package) {
        Some(rest) if rest.starts_with('.') && rest.len() > 1 => rest.to_owned(),
        _ => class.to_owned(),
    }
}

impl DexProgram {
    // cite: frameworks-base ComponentName.java#ComponentName, mPackage, mClass
    pub(in crate::dex_run) fn new_component_name(&self, package: &str, class: &str) -> DexValue {
        let component = self.allocate_object("Landroid/content/ComponentName;".to_owned());
        self.write_component_name(&component, package, class);
        component
    }

    fn write_component_name(&self, component: &DexValue, package: &str, class: &str) {
        for (field, value) in [("mPackage", package), ("mClass", class)] {
            self.write_stored_instance_field(
                component,
                &component_name_field(field),
                DexValue::String(std::rc::Rc::from(value)),
            );
        }
    }

    // cite: frameworks-base ComponentName.java#ComponentName, getPackageName, getClassName, getShortClassName, flattenToString, flattenToShortString, toString
    fn component_name_method(
        &self,
        method: &DexMethodId,
        invocation: &DexInvocation,
    ) -> Option<DexExecutionResult> {
        let arguments = invocation.arguments.as_slice();
        let component = arguments.first()?;
        if !dex_value_is_object_like(component) {
            return None;
        }
        let descriptor = method.proto.descriptor();
        if method.name == "<init>" {
            let package_name = || self.package_name.clone().unwrap_or_default();
            let (package, class) = match (descriptor.as_str(), arguments.get(1), arguments.get(2)) {
                (
                    "(Ljava/lang/String;Ljava/lang/String;)V",
                    Some(DexValue::String(package)),
                    Some(DexValue::String(class)),
                ) => (package.to_string(), class.to_string()),
                (
                    "(Landroid/content/Context;Ljava/lang/String;)V",
                    Some(_),
                    Some(DexValue::String(class)),
                ) => (package_name(), class.to_string()),
                ("(Landroid/content/Context;Ljava/lang/Class;)V", Some(_), Some(class)) => {
                    let descriptor = class_reference_descriptor_from_value(class)?;
                    (package_name(), descriptor_to_java_name(descriptor)?)
                }
                (_, _, Some(DexValue::Null | DexValue::Int(0))) => {
                    return Some(self.null_pointer_exception_result());
                }
                _ => return None,
            };
            self.write_component_name(component, &package, &class);
            return Some(DexExecutionResult::Void);
        }

        let read = |name: &str| match self
            .read_stored_instance_field(component, &component_name_field(name))
        {
            Some(DexValue::String(value)) => Some(value.to_string()),
            _ => None,
        };
        let (package, class) = (read("mPackage")?, read("mClass")?);
        let text = match (method.name.as_str(), descriptor.as_str()) {
            ("getPackageName", "()Ljava/lang/String;") => package,
            ("getClassName", "()Ljava/lang/String;") => class,
            ("getShortClassName", "()Ljava/lang/String;") => {
                component_short_class_name(&package, &class)
            }
            ("flattenToString", "()Ljava/lang/String;") => format!("{package}/{class}"),
            ("flattenToShortString", "()Ljava/lang/String;") => {
                format!("{package}/{}", component_short_class_name(&package, &class))
            }
            ("toString", "()Ljava/lang/String;") => format!("ComponentInfo{{{package}/{class}}}"),
            _ => return None,
        };

        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from(text),
        )))
    }
}
