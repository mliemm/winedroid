use super::super::*;

impl DexProgram {
    pub(super) fn invoke_android_content_resources_builtin(
        &self,
        method: &DexMethodId,
        invocation: Option<&DexInvocation>,
    ) -> Option<DexExecutionResult> {
        if method.class_descriptor == "Landroid/app/UiModeManager;"
            && method.name == "getCurrentModeType"
            && method.proto.descriptor_is("()I")
        {
            return Some(DexExecutionResult::Int(1));
        }

        if method.class_descriptor == "Landroid/app/UiModeManager;"
            && method.name == "getNightMode"
            && method.proto.descriptor_is("()I")
        {
            return Some(DexExecutionResult::Int(1));
        }

        if method.class_descriptor == "Landroid/app/AppOpsManager;"
            && method.name == "permissionToOp"
            && method
                .proto
                .descriptor_is("(Ljava/lang/String;)Ljava/lang/String;")
        {
            return Some(DexExecutionResult::Value(DexValue::Null));
        }

        if method.class_descriptor == "Landroid/location/LocationManager;"
            && method.name == "isProviderEnabled"
            && method.proto.descriptor_is("(Ljava/lang/String;)Z")
        {
            return Some(DexExecutionResult::Int(0));
        }

        if method.class_descriptor == "Landroid/content/res/ColorStateList;"
            && method.name == "valueOf"
            && method
                .proto
                .descriptor_is("(I)Landroid/content/res/ColorStateList;")
        {
            let invocation = invocation?;
            let value = self.color_state_list_value_of(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Landroid/content/res/ColorStateList;"
            && method.name == "getDefaultColor"
            && method.proto.descriptor_is("()I")
        {
            let invocation = invocation?;
            let value = self.color_state_list_get_default_color(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Landroid/content/Context;"
            && method.name == "getSharedPreferences"
            && method.proto.descriptor()
                == "(Ljava/lang/String;I)Landroid/content/SharedPreferences;"
        {
            let invocation = invocation?;
            // cite: frameworks-base ContextImpl.java#getSharedPreferencesPath, makeFilename
            if matches!(
                invocation.arguments.get(1),
                Some(DexValue::String(name)) if name.contains('/')
            ) {
                return Some(DexExecutionResult::Throw(
                    self.allocate_object("Ljava/lang/IllegalArgumentException;".to_owned()),
                ));
            }
            let preferences = self.shared_preferences_reference(invocation)?;

            return Some(DexExecutionResult::Value(preferences));
        }

        if (method.class_descriptor == "Landroid/preference/PreferenceManager;"
            || method.class_descriptor == "Landroidx/preference/PreferenceManager;")
            && method.name == "getDefaultSharedPreferences"
            && method.proto.descriptor()
                == "(Landroid/content/Context;)Landroid/content/SharedPreferences;"
        {
            let invocation = invocation?;
            let context = invocation.arguments.first()?;
            let preferences = self.default_shared_preferences_reference_for_context(context)?;

            return Some(DexExecutionResult::Value(preferences));
        }

        if method.class_descriptor == "Landroid/provider/Settings$Global;"
            && method.name == "getUriFor"
            && method
                .proto
                .descriptor_is("(Ljava/lang/String;)Landroid/net/Uri;")
        {
            let invocation = invocation?;
            let value = self.settings_global_get_uri_for(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if matches!(
            method.class_descriptor.as_str(),
            "Landroid/provider/Settings$Secure;"
                | "Landroid/provider/Settings$System;"
                | "Landroid/provider/Settings$Global;"
        ) && method.name == "getInt"
            && method.proto.descriptor()
                == "(Landroid/content/ContentResolver;Ljava/lang/String;I)I"
        {
            // cite: frameworks-base Settings.java#getInt
            let invocation = invocation?;
            let [resolver, DexValue::String(_), DexValue::Int(default)] =
                invocation.arguments.as_slice()
            else {
                return None;
            };
            if !dex_value_is_object_like(resolver) {
                return None;
            }

            return Some(DexExecutionResult::Int(*default));
        }

        if matches!(
            method.class_descriptor.as_str(),
            "Landroid/provider/Settings$Secure;"
                | "Landroid/provider/Settings$System;"
                | "Landroid/provider/Settings$Global;"
        ) && method.name == "getFloat"
            && method.proto.descriptor() == "(Landroid/content/ContentResolver;Ljava/lang/String;)F"
        {
            // cite: frameworks-base Settings.java#getFloat, parseFloatSetting, SettingNotFoundException
            let invocation = invocation?;
            let [resolver, DexValue::String(name)] = invocation.arguments.as_slice() else {
                return None;
            };
            if !dex_value_is_object_like(resolver) {
                return None;
            }
            let exception = self
                .allocate_object("Landroid/provider/Settings$SettingNotFoundException;".to_owned());
            self.write_stored_instance_field(
                &exception,
                &DexFieldId {
                    class_descriptor: "Ljava/lang/Throwable;".to_owned(),
                    type_descriptor: "Ljava/lang/String;".to_owned(),
                    name: "detailMessage".to_owned(),
                },
                DexValue::String(name.clone()),
            );

            return Some(DexExecutionResult::Throw(exception));
        }

        if method.class_descriptor == "Landroid/provider/Settings$Global;"
            && method.name == "getFloat"
            && method.proto.descriptor()
                == "(Landroid/content/ContentResolver;Ljava/lang/String;F)F"
        {
            let invocation = invocation?;
            let value = settings_global_get_float(invocation)?;

            return Some(DexExecutionResult::Value(DexValue::Float(value)));
        }

        if method.class_descriptor == "Landroid/content/Context;"
            && method.name == "getString"
            && method.proto.descriptor_is("(I)Ljava/lang/String;")
        {
            let invocation = invocation?;
            // cite: frameworks-base Context.java#getString
            let value = match self.context_string_resource(invocation) {
                Ok(Some(value)) => value,
                Ok(None) => {
                    return Some(DexExecutionResult::Throw(
                        self.resource_not_found_for_request(invocation, "string"),
                    ));
                }
                Err(error) => {
                    return Some(DexExecutionResult::Throw(
                        self.resource_not_found_exception(error),
                    ));
                }
            };

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Landroid/content/Context;"
            && method.name == "getText"
            && method.proto.descriptor_is("(I)Ljava/lang/CharSequence;")
        {
            let invocation = invocation?;
            // cite: frameworks-base Context.java#getText
            let value = match self.context_string_resource(invocation) {
                Ok(Some(value)) => value,
                Ok(None) => {
                    return Some(DexExecutionResult::Throw(
                        self.resource_not_found_for_request(invocation, "text"),
                    ));
                }
                Err(error) => {
                    return Some(DexExecutionResult::Throw(
                        self.resource_not_found_exception(error),
                    ));
                }
            };

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Landroid/content/Context;"
            && method.name == "getDrawable"
            && method
                .proto
                .descriptor_is("(I)Landroid/graphics/drawable/Drawable;")
        {
            let invocation = invocation?;
            let value = context_drawable_resource(self, invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Landroid/content/res/Resources;"
            && method.name == "getDrawable"
            && (method
                .proto
                .descriptor_is("(I)Landroid/graphics/drawable/Drawable;")
                || method.proto.descriptor_is(
                    "(ILandroid/content/res/Resources$Theme;)Landroid/graphics/drawable/Drawable;",
                ))
        {
            let invocation = invocation?;
            // cite: frameworks-base Resources.java#getDrawable
            // cite: androidx ImageResources.android.kt#imageResource
            let value = resources_drawable_resource(self, invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        // cite: frameworks-base ContextImpl.java#createConfigurationContext
        if method.class_descriptor == "Landroid/content/Context;"
            && method.name == "createConfigurationContext"
            && method
                .proto
                .descriptor_is("(Landroid/content/res/Configuration;)Landroid/content/Context;")
        {
            let invocation = invocation?;
            if invocation.arguments.len() != 2
                || !dex_value_is_object_like(&invocation.arguments[1])
            {
                return None;
            }
            let configuration_key = map_identity_key(&invocation.arguments[1]);

            return Some(DexExecutionResult::Value(DexValue::Reference(
                std::rc::Rc::from(format!(
                    "Landroid/content/Context;.configuration:{configuration_key}"
                )),
            )));
        }

        if method.class_descriptor == "Landroid/content/Context;"
            && method.name == "getColor"
            && method.proto.descriptor_is("(I)I")
        {
            let invocation = invocation?;
            // cite: frameworks-base Context.java#getColor
            match self.resources_get_color(invocation) {
                Ok(Some(result)) => return Some(result),
                Ok(None) => return None,
                Err(error) => {
                    return Some(DexExecutionResult::Throw(
                        self.resource_not_found_exception(error),
                    ));
                }
            }
        }

        // cite: frameworks-base ContextImpl.java#getMainExecutor
        if method.class_descriptor == "Landroid/content/Context;"
            && method.name == "getMainExecutor"
            && method
                .proto
                .descriptor_is("()Ljava/util/concurrent/Executor;")
        {
            return Some(DexExecutionResult::Value(DexValue::Reference(
                std::rc::Rc::from("Ljava/util/concurrent/Executor;.main"),
            )));
        }

        if method.class_descriptor == "Landroid/content/Context;"
            && method.name == "getMainExecutor"
            && method
                .proto
                .descriptor_is("()Ljava/util/concurrent/Executor;")
        {
            // cite: frameworks-base ContextImpl.java#getMainExecutor
            return Some(DexExecutionResult::Value(DexValue::Reference(
                std::rc::Rc::from("Ljava/util/concurrent/Executor;.main"),
            )));
        }

        if method.class_descriptor == "Landroid/content/Context;"
            && method.name == "getResources"
            && method
                .proto
                .descriptor_is("()Landroid/content/res/Resources;")
        {
            return Some(DexExecutionResult::Value(DexValue::Reference(
                std::rc::Rc::from("Landroid/content/res/Resources;.bootstrap"),
            )));
        }

        if method.class_descriptor == "Landroid/content/Context;"
            && method.name == "getAssets"
            && method
                .proto
                .descriptor_is("()Landroid/content/res/AssetManager;")
        {
            // cite: frameworks-base ContextImpl.java#getAssets
            return Some(DexExecutionResult::Value(DexValue::Reference(
                std::rc::Rc::from("Landroid/content/res/AssetManager;.bootstrap"),
            )));
        }

        if method.class_descriptor == "Landroid/content/res/Resources;"
            && method.name == "getAssets"
            && method
                .proto
                .descriptor_is("()Landroid/content/res/AssetManager;")
        {
            // cite: frameworks-base Resources.java#getAssets
            // cite: frameworks-base ResourcesImpl.java#getAssets
            return Some(DexExecutionResult::Value(DexValue::Reference(
                std::rc::Rc::from("Landroid/content/res/AssetManager;.bootstrap"),
            )));
        }

        if method.class_descriptor == "Landroid/content/res/AssetManager;"
            && method.name == "open"
            && method
                .proto
                .descriptor_is("(Ljava/lang/String;)Ljava/io/InputStream;")
        {
            let invocation = invocation?;

            return self.open_asset(invocation);
        }

        if method.class_descriptor == "Landroid/content/res/Resources;"
            && method.name == "getSystem"
            && method
                .proto
                .descriptor_is("()Landroid/content/res/Resources;")
        {
            // cite: frameworks-base Resources.java#getSystem
            return Some(DexExecutionResult::Value(DexValue::Reference(
                std::rc::Rc::from("Landroid/content/res/Resources;.bootstrap"),
            )));
        }

        if method.class_descriptor == "Landroid/content/res/Resources;"
            && method.name == "openRawResource"
            && method.proto.descriptor_is("(I)Ljava/io/InputStream;")
        {
            let invocation = invocation?;
            let value = self.open_raw_resource(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Landroid/util/Xml;"
            && method.name == "newPullParser"
            && method
                .proto
                .descriptor_is("()Lorg/xmlpull/v1/XmlPullParser;")
        {
            return Some(DexExecutionResult::Value(self.xml_new_pull_parser()));
        }

        if method.class_descriptor == "Lorg/xml/sax/helpers/DefaultHandler;"
            && method.name == "<init>"
            && method.proto.descriptor_is("()V")
        {
            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Lorg/xmlpull/v1/XmlPullParser;"
            && method.name == "setInput"
            && method
                .proto
                .descriptor_is("(Ljava/io/InputStream;Ljava/lang/String;)V")
        {
            let invocation = invocation?;
            self.xml_pull_parser_set_input(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/content/res/Resources;"
            && method.name == "getIdentifier"
            && method.proto.descriptor()
                == "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)I"
        {
            let invocation = invocation?;
            let name = match invocation.arguments.get(1) {
                Some(DexValue::String(name)) => name.as_ref(),
                _ => "",
            };
            let def_type = match invocation.arguments.get(2) {
                Some(DexValue::String(def_type)) => def_type.as_ref(),
                _ => "",
            };
            let def_package = match invocation.arguments.get(3) {
                Some(DexValue::String(def_package)) => def_package.as_ref(),
                _ => "",
            };

            return Some(DexExecutionResult::Int(self.resource_identifier(
                name,
                def_type,
                def_package,
            )));
        }

        if method.class_descriptor == "Landroid/content/Context;"
            && method.name == "getContentResolver"
            && method
                .proto
                .descriptor_is("()Landroid/content/ContentResolver;")
        {
            return Some(DexExecutionResult::Value(DexValue::Reference(
                std::rc::Rc::from("Landroid/content/ContentResolver;.bootstrap"),
            )));
        }

        if method.class_descriptor == "Landroid/content/ContentResolver;"
            && method.name == "registerContentObserver"
            && method.proto.descriptor()
                == "(Landroid/net/Uri;ZLandroid/database/ContentObserver;)V"
        {
            let invocation = invocation?;
            register_content_observer(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        // cite: frameworks-base ContentResolver.java#notifyChange
        if method.class_descriptor == "Landroid/content/ContentResolver;"
            && method.name == "notifyChange"
            && method
                .proto
                .descriptor_is("(Landroid/net/Uri;Landroid/database/ContentObserver;)V")
        {
            let invocation = invocation?;
            let [resolver, uri, observer] = invocation.arguments.as_slice() else {
                return None;
            };
            if !dex_value_is_object_like(resolver)
                || !dex_value_is_object_like(uri)
                || !dex_value_is_nullable_object_like(observer)
            {
                return None;
            }
            return Some(DexExecutionResult::Void);
        }

        // cite: frameworks-base ContentResolver.java#unregisterContentObserver
        if method.class_descriptor == "Landroid/content/ContentResolver;"
            && method.name == "unregisterContentObserver"
            && method
                .proto
                .descriptor_is("(Landroid/database/ContentObserver;)V")
        {
            let invocation = invocation?;
            unregister_content_observer(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/content/Context;"
            && method.name == "getApplicationInfo"
            && method
                .proto
                .descriptor_is("()Landroid/content/pm/ApplicationInfo;")
        {
            return Some(DexExecutionResult::Value(DexValue::Reference(
                std::rc::Rc::from("Landroid/content/pm/ApplicationInfo;.bootstrap"),
            )));
        }

        if method.class_descriptor == "Landroid/view/ContextThemeWrapper;"
            && method.name == "<init>"
            && method.proto.descriptor_is("(Landroid/content/Context;I)V")
        {
            let invocation = invocation?;
            self.context_theme_wrapper_init(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.matches_builtin_signature(
            "Landroid/view/ContextThemeWrapper;",
            "<init>",
            "(Landroid/content/Context;Landroid/content/res/Resources$Theme;)V",
        ) {
            // cite: frameworks-base ContextThemeWrapper.java#ContextThemeWrapper, mTheme
            let invocation = invocation?;
            let [wrapper, base, theme] = invocation.arguments.as_slice() else {
                return None;
            };
            if !dex_value_is_object_like(wrapper)
                || !dex_value_is_nullable_object_like(base)
                || !dex_value_is_nullable_object_like(theme)
            {
                return None;
            }
            if dex_value_is_object_like(base) {
                self.write_stored_instance_field(
                    wrapper,
                    &DexFieldId {
                        class_descriptor: "Landroid/content/ContextWrapper;".to_owned(),
                        type_descriptor: "Landroid/content/Context;".to_owned(),
                        name: "mBase".to_owned(),
                    },
                    base.clone(),
                );
            }
            if dex_value_is_object_like(theme) {
                self.write_stored_instance_field(
                    wrapper,
                    &context_theme_wrapper_theme_field(),
                    theme.clone(),
                );
            }

            return Some(DexExecutionResult::Void);
        }

        if matches!(
            method.class_descriptor.as_str(),
            "Landroid/content/ContextWrapper;" | "Landroid/content/MutableContextWrapper;"
        ) && (method.name == "<init>"
            || method.class_descriptor == "Landroid/content/MutableContextWrapper;"
                && method.name == "setBaseContext")
            && method.proto.descriptor_is("(Landroid/content/Context;)V")
        {
            // cite: frameworks-base ContextWrapper.java#ContextWrapper, mBase
            // cite: frameworks-base MutableContextWrapper.java#MutableContextWrapper, setBaseContext
            let invocation = invocation?;
            let [receiver, base] = invocation.arguments.as_slice() else {
                return None;
            };
            if !dex_value_is_object_like(receiver) || !dex_value_is_nullable_object_like(base) {
                return None;
            }
            let base_field = DexFieldId {
                class_descriptor: "Landroid/content/ContextWrapper;".to_owned(),
                type_descriptor: "Landroid/content/Context;".to_owned(),
                name: "mBase".to_owned(),
            };
            if dex_value_is_object_like(base) {
                self.write_stored_instance_field(receiver, &base_field, base.clone());
            } else {
                self.remove_stored_instance_field(receiver, &base_field);
            }

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/content/ContextWrapper;"
            && method.name == "attachBaseContext"
            && method.proto.descriptor_is("(Landroid/content/Context;)V")
        {
            let invocation = invocation?;

            return self.context_wrapper_attach_base_context(invocation);
        }

        if method.class_descriptor == "Landroid/content/ContextWrapper;"
            && method.name == "getBaseContext"
            && method.proto.descriptor_is("()Landroid/content/Context;")
        {
            let invocation = invocation?;
            let value = self.context_wrapper_base_context(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Landroid/content/res/Resources;"
            && method.name == "getDimension"
            && method.proto.descriptor_is("(I)F")
        {
            let invocation = invocation?;
            // cite: frameworks-base Resources.java#getDimension
            let value = match self.resources_get_dimension(invocation) {
                Ok(Some(value)) => value,
                Ok(None) => {
                    return Some(DexExecutionResult::Throw(
                        self.resource_not_found_for_request(invocation, "dimension"),
                    ));
                }
                Err(error) => {
                    return Some(DexExecutionResult::Throw(
                        self.resource_not_found_exception(error),
                    ));
                }
            };

            return Some(DexExecutionResult::Value(DexValue::Float(value)));
        }

        if method.class_descriptor == "Landroid/content/res/Resources;"
            && method.name == "getDimensionPixelOffset"
            && method.proto.descriptor_is("(I)I")
        {
            let invocation = invocation?;
            // cite: frameworks-base Resources.java#getDimensionPixelOffset
            let value = match self.resources_get_dimension_pixel_offset(invocation) {
                Ok(Some(value)) => value,
                Ok(None) => {
                    return Some(DexExecutionResult::Throw(
                        self.resource_not_found_for_request(invocation, "dimension"),
                    ));
                }
                Err(error) => {
                    return Some(DexExecutionResult::Throw(
                        self.resource_not_found_exception(error),
                    ));
                }
            };

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Landroid/content/res/Resources;"
            && method.name == "getDimensionPixelSize"
            && method.proto.descriptor_is("(I)I")
        {
            let invocation = invocation?;
            // cite: frameworks-base Resources.java#getDimensionPixelSize
            let value = match self.resources_get_dimension_pixel_size(invocation) {
                Ok(Some(value)) => value,
                Ok(None) => {
                    return Some(DexExecutionResult::Throw(
                        self.resource_not_found_for_request(invocation, "dimension"),
                    ));
                }
                Err(error) => {
                    return Some(DexExecutionResult::Throw(
                        self.resource_not_found_exception(error),
                    ));
                }
            };

            return Some(DexExecutionResult::Int(value));
        }

        if method.matches_builtin_signature("Landroid/content/res/Resources;", "getInteger", "(I)I")
        {
            let invocation = invocation?;
            // cite: frameworks-base Resources.java#getInteger
            let value = match self.resources_get_integer(invocation) {
                Ok(Some(value)) => value,
                Ok(None) => {
                    return Some(DexExecutionResult::Throw(
                        self.resource_not_found_for_request(invocation, "integer"),
                    ));
                }
                Err(error) => {
                    return Some(DexExecutionResult::Throw(
                        self.resource_not_found_exception(error),
                    ));
                }
            };

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Landroid/content/res/Resources;"
            && method.name == "getBoolean"
            && method.proto.descriptor_is("(I)Z")
        {
            let invocation = invocation?;
            // cite: frameworks-base Resources.java#getBoolean
            let value = match self.resources_get_boolean(invocation) {
                Ok(Some(value)) => value,
                Ok(None) => {
                    return Some(DexExecutionResult::Throw(
                        self.resource_not_found_for_request(invocation, "boolean"),
                    ));
                }
                Err(error) => {
                    return Some(DexExecutionResult::Throw(
                        self.resource_not_found_exception(error),
                    ));
                }
            };

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Landroid/content/res/Resources;"
            && method.name == "getString"
            && method.proto.descriptor_is("(I)Ljava/lang/String;")
        {
            let invocation = invocation?;
            // cite: frameworks-base Resources.java#getString
            let value = match self.context_string_resource(invocation) {
                Ok(Some(value)) => value,
                Ok(None) => {
                    return Some(DexExecutionResult::Throw(
                        self.resource_not_found_for_request(invocation, "string"),
                    ));
                }
                Err(error) => {
                    return Some(DexExecutionResult::Throw(
                        self.resource_not_found_exception(error),
                    ));
                }
            };

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Landroid/content/res/Resources;"
            && method.name == "getStringArray"
            && method.proto.descriptor_is("(I)[Ljava/lang/String;")
        {
            let invocation = invocation?;
            // cite: frameworks-base Resources.java#getStringArray
            let value = match self.resources_get_string_array(invocation) {
                Ok(Some(value)) => value,
                Ok(None) => {
                    return Some(DexExecutionResult::Throw(
                        self.resource_not_found_for_request(invocation, "string array"),
                    ));
                }
                Err(error) => {
                    return Some(DexExecutionResult::Throw(
                        self.resource_not_found_exception(error),
                    ));
                }
            };

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Landroid/content/res/Resources;"
            && method.name == "getText"
            && method.proto.descriptor_is("(I)Ljava/lang/CharSequence;")
        {
            let invocation = invocation?;
            // cite: frameworks-base Resources.java#getText
            let value = match self.context_string_resource(invocation) {
                Ok(Some(value)) => value,
                Ok(None) => {
                    return Some(DexExecutionResult::Throw(
                        self.resource_not_found_for_request(invocation, "text"),
                    ));
                }
                Err(error) => {
                    return Some(DexExecutionResult::Throw(
                        self.resource_not_found_exception(error),
                    ));
                }
            };

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Landroid/content/res/Resources;"
            && method.name == "getColor"
            && method.proto.descriptor_is("(I)I")
        {
            let invocation = invocation?;
            // cite: frameworks-base Resources.java#getColor
            match self.resources_get_color(invocation) {
                Ok(Some(result)) => return Some(result),
                Ok(None) => return None,
                Err(error) => {
                    return Some(DexExecutionResult::Throw(
                        self.resource_not_found_exception(error),
                    ));
                }
            }
        }

        if method.class_descriptor == "Landroid/content/res/Resources;"
            && method.name == "getColor"
            && method
                .proto
                .descriptor_is("(ILandroid/content/res/Resources$Theme;)I")
        {
            let invocation = invocation?;
            // cite: frameworks-base Resources.java#getColor
            match self.resources_get_color(invocation) {
                Ok(Some(result)) => return Some(result),
                Ok(None) => return None,
                Err(error) => {
                    return Some(DexExecutionResult::Throw(
                        self.resource_not_found_exception(error),
                    ));
                }
            }
        }

        // cite: frameworks-base Resources.java#Resources
        if method.class_descriptor == "Landroid/content/res/Resources;"
            && method.name == "<init>"
            && method.proto.descriptor()
                == "(Landroid/content/res/AssetManager;Landroid/util/DisplayMetrics;Landroid/content/res/Configuration;)V"
        {
            let invocation = invocation?;
            if invocation.arguments.len() != 4
                || invocation
                    .arguments
                    .iter()
                    .any(|value| !dex_value_is_object_like(value))
            {
                return None;
            }

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/content/res/Resources;"
            && method.name == "getDisplayMetrics"
            && method
                .proto
                .descriptor_is("()Landroid/util/DisplayMetrics;")
        {
            return Some(DexExecutionResult::Value(DexValue::Reference(
                std::rc::Rc::from("Landroid/util/DisplayMetrics;.bootstrap"),
            )));
        }

        if method.class_descriptor == "Landroid/content/res/Resources;"
            && method.name == "getResourceName"
            && method.proto.descriptor_is("(I)Ljava/lang/String;")
        {
            let invocation = invocation?;
            let value = resource_name(invocation)?;

            return Some(DexExecutionResult::Value(DexValue::String(
                std::rc::Rc::from(value),
            )));
        }

        if method.class_descriptor == "Landroid/content/res/Resources;"
            && method.name == "getValue"
            && method.proto.descriptor_is("(ILandroid/util/TypedValue;Z)V")
        {
            let invocation = invocation?;
            // cite: frameworks-base Resources.java#getValue
            match self.resources_get_value(invocation) {
                Ok(Some(())) => {}
                Ok(None) => {
                    return Some(DexExecutionResult::Throw(
                        self.resource_not_found_for_request(invocation, "typed value"),
                    ));
                }
                Err(error) => {
                    return Some(DexExecutionResult::Throw(
                        self.resource_not_found_exception(error),
                    ));
                }
            }

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/content/res/Resources;"
            && method.name == "getLayout"
            && method
                .proto
                .descriptor_is("(I)Landroid/content/res/XmlResourceParser;")
        {
            let invocation = invocation?;
            let value = self.resources_get_layout(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Landroid/content/res/Resources;"
            && method.name == "getXml"
            && method
                .proto
                .descriptor_is("(I)Landroid/content/res/XmlResourceParser;")
        {
            let invocation = invocation?;
            let value = self.resources_get_layout(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Landroid/content/res/Resources;"
            && method.name == "getConfiguration"
            && method
                .proto
                .descriptor_is("()Landroid/content/res/Configuration;")
        {
            return Some(DexExecutionResult::Value(DexValue::Reference(
                std::rc::Rc::from("Landroid/content/res/Configuration;.bootstrap"),
            )));
        }

        if method.class_descriptor == "Landroid/content/res/Resources;"
            && method.name == "updateConfiguration"
            && method.proto.descriptor()
                == "(Landroid/content/res/Configuration;Landroid/util/DisplayMetrics;)V"
        {
            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/content/res/TypedArray;"
            && method.name == "hasValue"
            && method.proto.descriptor_is("(I)Z")
        {
            let invocation = invocation?;
            let value = self.typed_array_has_value(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Landroid/content/res/TypedArray;"
            && method.name == "getBoolean"
            && method.proto.descriptor_is("(IZ)Z")
        {
            let invocation = invocation?;
            let value = self.typed_array_get_boolean(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Landroid/content/res/TypedArray;"
            && method.name == "getIndexCount"
            && method.proto.descriptor_is("()I")
        {
            let invocation = invocation?;
            typed_array_receiver(invocation)?;

            return Some(DexExecutionResult::Int(0));
        }

        if method.class_descriptor == "Landroid/content/res/TypedArray;"
            && method.name == "getIndex"
            && method.proto.descriptor_is("(I)I")
        {
            let invocation = invocation?;
            let value = typed_array_index(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Landroid/content/res/TypedArray;"
            && method.name == "getChangingConfigurations"
            && method.proto.descriptor_is("()I")
        {
            let invocation = invocation?;
            typed_array_receiver(invocation)?;

            return Some(DexExecutionResult::Int(0));
        }

        if method.class_descriptor == "Landroid/content/res/TypedArray;"
            && method.name == "getResources"
            && method
                .proto
                .descriptor_is("()Landroid/content/res/Resources;")
        {
            let invocation = invocation?;
            typed_array_receiver(invocation)?;

            return Some(DexExecutionResult::Value(DexValue::Reference(
                std::rc::Rc::from("Landroid/content/res/Resources;.bootstrap"),
            )));
        }

        if method.class_descriptor == "Landroid/content/res/TypedArray;"
            && method.name == "getResourceId"
            && method.proto.descriptor_is("(II)I")
        {
            let invocation = invocation?;
            let value = self.typed_array_get_resource_id(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Landroid/content/res/TypedArray;"
            && method.name == "getInt"
            && method.proto.descriptor_is("(II)I")
        {
            let invocation = invocation?;
            let value = self.typed_array_get_int(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Landroid/content/res/TypedArray;"
            && method.name == "getColor"
            && method.proto.descriptor_is("(II)I")
        {
            let invocation = invocation?;
            // cite: frameworks-base TypedArray.java#getColor
            match self.typed_array_get_color(invocation) {
                Ok(Some(result)) => return Some(result),
                Ok(None) => return None,
                Err(error) => {
                    return Some(DexExecutionResult::Throw(
                        self.resource_not_found_exception(error),
                    ));
                }
            }
        }

        if method.class_descriptor == "Landroid/content/res/TypedArray;"
            && method.name == "getFloat"
            && method.proto.descriptor_is("(IF)F")
        {
            let invocation = invocation?;
            let value = self.typed_array_get_float(invocation)?;

            return Some(DexExecutionResult::Value(DexValue::Float(value)));
        }

        if method.class_descriptor == "Landroid/content/res/TypedArray;"
            && method.name == "getDimension"
            && method.proto.descriptor_is("(IF)F")
        {
            let invocation = invocation?;
            let value = self.typed_array_get_dimension(invocation)?;

            return Some(DexExecutionResult::Value(DexValue::Float(value)));
        }

        if method.class_descriptor == "Landroid/content/res/TypedArray;"
            && method.name == "getDimensionPixelOffset"
            && method.proto.descriptor_is("(II)I")
        {
            let invocation = invocation?;
            let value = self.typed_array_get_dimension_pixel_offset(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Landroid/content/res/TypedArray;"
            && method.name == "getDimensionPixelSize"
            && method.proto.descriptor_is("(II)I")
        {
            let invocation = invocation?;
            let value = self.typed_array_get_dimension_pixel_size(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Landroid/content/res/TypedArray;"
            && method.name == "getLayoutDimension"
            && method.proto.descriptor_is("(II)I")
        {
            let invocation = invocation?;
            let value = self.typed_array_get_layout_dimension(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Landroid/content/res/TypedArray;"
            && method.name == "getText"
            && method.proto.descriptor_is("(I)Ljava/lang/CharSequence;")
        {
            let invocation = invocation?;
            // cite: frameworks-base TypedArray.java#getText
            let value = match self.typed_array_get_text(invocation) {
                Ok(Some(value)) => value,
                Ok(None) => {
                    return Some(DexExecutionResult::Throw(
                        self.resource_not_found_for_request(invocation, "text"),
                    ));
                }
                Err(error) => {
                    return Some(DexExecutionResult::Throw(
                        self.resource_not_found_exception(error),
                    ));
                }
            };

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Landroid/content/res/TypedArray;"
            && method.name == "getString"
            && method.proto.descriptor_is("(I)Ljava/lang/String;")
        {
            let invocation = invocation?;
            // cite: frameworks-base TypedArray.java#getString
            let value = match self.typed_array_get_text(invocation) {
                Ok(Some(value)) => value,
                Ok(None) => {
                    return Some(DexExecutionResult::Throw(
                        self.resource_not_found_for_request(invocation, "string"),
                    ));
                }
                Err(error) => {
                    return Some(DexExecutionResult::Throw(
                        self.resource_not_found_exception(error),
                    ));
                }
            };

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Landroid/content/res/TypedArray;"
            && method.name == "getTextArray"
            && method.proto.descriptor_is("(I)[Ljava/lang/CharSequence;")
        {
            let invocation = invocation?;
            // cite: frameworks-base TypedArray.java#getTextArray
            let value = match self.typed_array_get_text_array(invocation) {
                Ok(Some(value)) => value,
                Ok(None) => {
                    return Some(DexExecutionResult::Throw(
                        self.resource_not_found_for_request(invocation, "text array"),
                    ));
                }
                Err(error) => {
                    return Some(DexExecutionResult::Throw(
                        self.resource_not_found_exception(error),
                    ));
                }
            };

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Landroid/content/res/TypedArray;"
            && method.name == "getColorStateList"
            && method
                .proto
                .descriptor_is("(I)Landroid/content/res/ColorStateList;")
        {
            let invocation = invocation?;
            match self.typed_array_get_color_state_list(invocation) {
                Ok(Some(result)) => return Some(result),
                Ok(None) => return None,
                Err(error) => {
                    return Some(DexExecutionResult::Throw(
                        self.resource_not_found_exception(error),
                    ));
                }
            }
        }

        if method.class_descriptor == "Landroid/content/res/TypedArray;"
            && method.name == "getDrawable"
            && method
                .proto
                .descriptor_is("(I)Landroid/graphics/drawable/Drawable;")
        {
            let invocation = invocation?;
            let value = self.typed_array_get_drawable(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Landroid/content/res/TypedArray;"
            && method.name == "getValue"
            && method.proto.descriptor_is("(ILandroid/util/TypedValue;)Z")
        {
            let invocation = invocation?;
            // cite: frameworks-base TypedArray.java#getValue
            let value = match self.typed_array_get_value(invocation) {
                Ok(Some(value)) => value,
                Ok(None) => {
                    return Some(DexExecutionResult::Throw(
                        self.resource_not_found_for_request(invocation, "typed value"),
                    ));
                }
                Err(error) => {
                    return Some(DexExecutionResult::Throw(
                        self.resource_not_found_exception(error),
                    ));
                }
            };

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Landroid/content/res/TypedArray;"
            && method.name == "recycle"
            && method.proto.descriptor_is("()V")
        {
            let invocation = invocation?;
            typed_array_receiver(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/content/res/Resources$Theme;"
            && method.name == "resolveAttribute"
            && method.proto.descriptor_is("(ILandroid/util/TypedValue;Z)Z")
        {
            let invocation = invocation?;
            let resolved = match self.theme_resolve_attribute(invocation) {
                Ok(Some(value)) => value,
                Ok(None) => {
                    return Some(DexExecutionResult::Throw(
                        self.resource_not_found_for_request(invocation, "theme attribute"),
                    ));
                }
                Err(error) => {
                    return Some(DexExecutionResult::Throw(
                        self.resource_not_found_exception(error),
                    ));
                }
            };

            return Some(DexExecutionResult::Int(resolved));
        }

        if method.class_descriptor == "Landroid/content/res/Resources$Theme;"
            && method.name == "applyStyle"
            && method.proto.descriptor_is("(IZ)V")
        {
            let invocation = invocation?;
            self.theme_apply_style(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/util/TypedValue;"
            && method.name == "<init>"
            && method.proto.descriptor_is("()V")
        {
            return Some(DexExecutionResult::Void);
        }

        if method.matches_builtin_signature(
            "Landroid/util/TypedValue;",
            "applyDimension",
            "(IFLandroid/util/DisplayMetrics;)F",
        ) {
            let invocation = invocation?;
            let value = self.typed_value_apply_dimension(invocation)?;

            return Some(DexExecutionResult::Value(DexValue::Float(value)));
        }

        if method.class_descriptor == "Landroid/util/Xml;"
            && method.name == "asAttributeSet"
            && method.proto.descriptor()
                == "(Lorg/xmlpull/v1/XmlPullParser;)Landroid/util/AttributeSet;"
        {
            let invocation = invocation?;
            let value = xml_as_attribute_set(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Lorg/xmlpull/v1/XmlPullParser;"
            && method.name == "getEventType"
            && method.proto.descriptor_is("()I")
        {
            let invocation = invocation?;
            let value = self.xml_pull_parser_event_type(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Lorg/xmlpull/v1/XmlPullParser;"
            && method.name == "next"
            && method.proto.descriptor_is("()I")
        {
            let invocation = invocation?;
            let value = self.xml_pull_parser_next(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Lorg/xmlpull/v1/XmlPullParser;"
            && method.name == "getName"
            && method.proto.descriptor_is("()Ljava/lang/String;")
        {
            let invocation = invocation?;
            let value = self.xml_pull_parser_name(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Lorg/xmlpull/v1/XmlPullParser;"
            && method.name == "getDepth"
            && method.proto.descriptor_is("()I")
        {
            let invocation = invocation?;
            let value = self.xml_pull_parser_depth(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Lorg/xmlpull/v1/XmlPullParser;"
            && method.name == "getAttributeValue"
            && method.proto.descriptor()
                == "(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;"
        {
            let invocation = invocation?;
            let value = self.xml_pull_parser_attribute_value(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Landroid/content/res/XmlResourceParser;"
            && method.name == "close"
            && method.proto.descriptor_is("()V")
        {
            let invocation = invocation?;
            self.xml_resource_parser_close(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/content/res/Configuration;"
            && method.name == "<init>"
            && method.proto.descriptor_is("()V")
        {
            if let Some(invocation) = invocation {
                self.configuration_init_default(invocation)?;
            }

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/content/res/Configuration;"
            && method.name == "<init>"
            && method
                .proto
                .descriptor_is("(Landroid/content/res/Configuration;)V")
        {
            if let Some(invocation) = invocation {
                self.configuration_copy_from(invocation)?;
            }

            return Some(DexExecutionResult::Void);
        }

        // cite: frameworks-base Configuration.java#setTo
        if method.class_descriptor == "Landroid/content/res/Configuration;"
            && method.name == "setTo"
            && method
                .proto
                .descriptor_is("(Landroid/content/res/Configuration;)V")
        {
            let invocation = invocation?;
            self.configuration_copy_from(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/content/res/Configuration;"
            && method.name == "updateFrom"
            && method
                .proto
                .descriptor_is("(Landroid/content/res/Configuration;)I")
        {
            let invocation = invocation?;
            let value = self.configuration_update_from(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Landroid/content/res/Configuration;"
            && method.name == "setLocale"
            && method.proto.descriptor_is("(Ljava/util/Locale;)V")
        {
            let invocation = invocation?;
            self.configuration_set_locale(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/content/res/Configuration;"
            && method.name == "getLocales"
            && method.proto.descriptor_is("()Landroid/os/LocaleList;")
        {
            let invocation = invocation?;
            let value = self.configuration_get_locales(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Landroid/content/res/Configuration;"
            && method.name == "getLayoutDirection"
            && method.proto.descriptor_is("()I")
        {
            let invocation = invocation?;
            let value = self.configuration_layout_direction(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Landroid/os/LocaleList;" {
            let stored = invocation.and_then(|invocation| {
                let array = self.locale_list_array(invocation.arguments.first()?)?;

                Some((invocation, array))
            });
            if let Some((invocation, array)) = stored {
                return self.stored_locale_list_call(method, invocation, &array);
            }
        }

        if method.class_descriptor == "Landroid/os/LocaleList;"
            && method.name == "getDefault"
            && method.proto.descriptor_is("()Landroid/os/LocaleList;")
        {
            return Some(DexExecutionResult::Value(DexValue::Reference(
                std::rc::Rc::from("Landroid/os/LocaleList;.bootstrap"),
            )));
        }

        if method.class_descriptor == "Landroid/os/LocaleList;"
            && method.name == "toLanguageTags"
            && method.proto.descriptor_is("()Ljava/lang/String;")
        {
            return Some(DexExecutionResult::Value(DexValue::String(
                std::rc::Rc::from("en-US"),
            )));
        }

        if method.class_descriptor == "Landroid/os/LocaleList;"
            && method.name == "isEmpty"
            && method.proto.descriptor_is("()Z")
        {
            return Some(DexExecutionResult::Int(0));
        }

        if method.class_descriptor == "Landroid/os/LocaleList;"
            && method.name == "size"
            && method.proto.descriptor_is("()I")
        {
            return Some(DexExecutionResult::Int(1));
        }

        if method.class_descriptor == "Landroid/os/LocaleList;"
            && method.name == "get"
            && method.proto.descriptor_is("(I)Ljava/util/Locale;")
        {
            let invocation = invocation?;
            let index = match invocation.arguments.as_slice() {
                [_, DexValue::Int(index)] => *index,
                _ => return None,
            };
            if index != 0 {
                return None;
            }

            return Some(DexExecutionResult::Value(DexValue::Reference(
                std::rc::Rc::from("Ljava/util/Locale;:default"),
            )));
        }

        if method.class_descriptor == "Landroid/os/LocaleList;"
            && method.name == "<init>"
            && method.proto.descriptor_is("([Ljava/util/Locale;)V")
        {
            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/content/Context;"
            && method.name == "getSystemService"
            && method
                .proto
                .descriptor_is("(Ljava/lang/String;)Ljava/lang/Object;")
        {
            let invocation = invocation?;
            if matches!(invocation.arguments.get(1), Some(DexValue::String(name)) if name.as_ref() == "layout_inflater")
            {
                return Some(DexExecutionResult::Value(
                    self.layout_inflater_for_context(invocation.arguments.first()?),
                ));
            }
            let value = context_system_service(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Landroid/content/Context;"
            && method.name == "getSystemService"
            && method
                .proto
                .descriptor_is("(Ljava/lang/Class;)Ljava/lang/Object;")
        {
            let invocation = invocation?;
            if invocation
                .arguments
                .get(1)
                .and_then(class_reference_descriptor_from_value)
                == Some("Landroid/view/LayoutInflater;")
            {
                return Some(DexExecutionResult::Value(
                    self.layout_inflater_for_context(invocation.arguments.first()?),
                ));
            }
            let value = context_system_service_by_class(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Landroid/content/Context;"
            && method.name == "obtainStyledAttributes"
            && method
                .proto
                .descriptor_is("([I)Landroid/content/res/TypedArray;")
        {
            let invocation = invocation?;
            let value = self.context_obtain_styled_attributes(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Landroid/content/Context;"
            && method.name == "obtainStyledAttributes"
            && method.proto.descriptor()
                == "(Landroid/util/AttributeSet;[I)Landroid/content/res/TypedArray;"
        {
            let invocation = invocation?;
            let value = context_obtain_attribute_set_styled_attributes(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Landroid/content/Context;"
            && method.name == "obtainStyledAttributes"
            && method.proto.descriptor()
                == "(Landroid/util/AttributeSet;[III)Landroid/content/res/TypedArray;"
        {
            let invocation = invocation?;
            let value = context_obtain_attribute_set_styled_attributes(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Landroid/content/Context;"
            && method.name == "getTheme"
            && method
                .proto
                .descriptor_is("()Landroid/content/res/Resources$Theme;")
        {
            let value = self.context_get_theme(invocation);

            return Some(DexExecutionResult::Value(value?));
        }

        if method.class_descriptor == "Landroid/content/res/Resources$Theme;"
            && method.name == "obtainStyledAttributes"
            && method
                .proto
                .descriptor_is("([I)Landroid/content/res/TypedArray;")
        {
            let invocation = invocation?;
            let value = self.theme_obtain_styled_attributes(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Landroid/content/res/Resources$Theme;"
            && method.name == "obtainStyledAttributes"
            && method.proto.descriptor()
                == "(Landroid/util/AttributeSet;[III)Landroid/content/res/TypedArray;"
        {
            let invocation = invocation?;
            let value = context_obtain_attribute_set_styled_attributes(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Landroid/content/Context;"
            && method.name == "getPackageName"
            && method.proto.descriptor_is("()Ljava/lang/String;")
        {
            return Some(DexExecutionResult::Value(DexValue::String(
                std::rc::Rc::from(self.bootstrap_process_name()),
            )));
        }

        if method.class_descriptor == "Landroid/content/Context;"
            && method.name == "checkPermission"
            && method.proto.descriptor_is("(Ljava/lang/String;II)I")
        {
            let invocation = invocation?;
            let DexValue::String(permission) = invocation.arguments.get(1)? else {
                return None;
            };
            // cite: frameworks-base ContextImpl.java#checkPermission, checkSelfPermission
            let storage_permission = self.external_storage_policy.permission_granted(permission);
            if storage_permission.is_some()
                && invocation.arguments.get(2)? != &DexValue::Int(BOOTSTRAP_PROCESS_ID)
            {
                return None;
            }
            if storage_permission.is_some()
                && invocation.arguments.get(3)? != &DexValue::Int(BOOTSTRAP_PROCESS_UID)
            {
                return None;
            }
            let result = storage_permission.map_or(ANDROID_PERMISSION_GRANTED, |granted| {
                if granted {
                    ANDROID_PERMISSION_GRANTED
                } else {
                    ANDROID_PERMISSION_DENIED
                }
            });
            return Some(DexExecutionResult::Int(result));
        }

        if method.class_descriptor == "Landroid/content/Context;"
            && method.name == "getPackageManager"
            && method
                .proto
                .descriptor_is("()Landroid/content/pm/PackageManager;")
        {
            return Some(DexExecutionResult::Value(DexValue::Reference(
                std::rc::Rc::from("Landroid/content/pm/PackageManager;.bootstrap"),
            )));
        }

        if method.class_descriptor == "Landroid/content/Context;"
            && method.name == "getClassLoader"
            && method.proto.descriptor_is("()Ljava/lang/ClassLoader;")
        {
            // cite: frameworks-base ContextImpl.java#getClassLoader
            return Some(DexExecutionResult::Value(DexValue::Reference(
                std::rc::Rc::from("Ljava/lang/ClassLoader;.app"),
            )));
        }

        if method.class_descriptor == "Landroid/content/Context;"
            && method.name == "getMainLooper"
            && method.proto.descriptor_is("()Landroid/os/Looper;")
        {
            return Some(DexExecutionResult::Value(DexValue::Reference(
                std::rc::Rc::from("Landroid/os/Looper;:main"),
            )));
        }

        None
    }
}

fn settings_global_get_float(invocation: &DexInvocation) -> Option<f32> {
    if invocation.arguments.len() != 3 {
        return None;
    }

    if !dex_value_is_object_like(&invocation.arguments[0]) {
        return None;
    }
    let DexValue::String(_) = &invocation.arguments[1] else {
        return None;
    };

    match invocation.arguments[2] {
        DexValue::Float(value) => Some(value),
        DexValue::Int(bits) => Some(f32::from_bits(bits as u32)),
        _ => None,
    }
}

// cite: frameworks-base ContentResolver.java#unregisterContentObserver
fn unregister_content_observer(invocation: &DexInvocation) -> Option<()> {
    if invocation.arguments.len() != 2 {
        return None;
    }

    if !dex_value_is_object_like(&invocation.arguments[0])
        || !dex_value_is_object_like(&invocation.arguments[1])
    {
        return None;
    }

    Some(())
}

fn register_content_observer(invocation: &DexInvocation) -> Option<()> {
    if invocation.arguments.len() != 4 {
        return None;
    }

    if !dex_value_is_object_like(&invocation.arguments[0])
        || !dex_value_is_object_like(&invocation.arguments[1])
        || !matches!(invocation.arguments[2], DexValue::Int(_))
        || !dex_value_is_object_like(&invocation.arguments[3])
    {
        return None;
    }

    Some(())
}

impl DexProgram {
    fn settings_global_get_uri_for(&self, invocation: &DexInvocation) -> Option<DexValue> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        let DexValue::String(name) = &invocation.arguments[0] else {
            return None;
        };
        let uri_invocation = DexInvocation {
            arguments: smallvec::smallvec![DexValue::String(std::rc::Rc::from(format!(
                "content://settings/global/{name}"
            )))],
            ..invocation.clone()
        };

        self.parse_uri(&uri_invocation)
    }
}

impl DexProgram {
    // cite: frameworks-base TypedValue.java#applyDimension
    fn typed_value_apply_dimension(&self, invocation: &DexInvocation) -> Option<f32> {
        let [DexValue::Int(unit), value, metrics] = invocation.arguments.as_slice() else {
            return None;
        };
        let value = match value {
            DexValue::Float(value) => *value,
            DexValue::Int(bits) => f32::from_bits(*bits as u32),
            _ => return None,
        };
        if !dex_value_is_object_like(metrics) {
            return None;
        }
        let metric = |name: &str| {
            let field = DexFieldId {
                class_descriptor: "Landroid/util/DisplayMetrics;".to_owned(),
                type_descriptor: "F".to_owned(),
                name: name.to_owned(),
            };
            match self
                .read_stored_instance_field(metrics, &field)
                .or_else(|| self.read_bootstrap_instance_field(metrics, &field))?
            {
                DexValue::Float(value) => Some(value),
                DexValue::Int(bits) => Some(f32::from_bits(bits as u32)),
                _ => None,
            }
        };

        Some(match unit {
            0 => value,
            1 => value * metric("density")?,
            2 => value * metric("scaledDensity")?,
            3 => value * metric("xdpi")? * (1.0 / 72.0),
            4 => value * metric("xdpi")?,
            5 => value * metric("xdpi")? * (1.0 / 25.4),
            _ => 0.0,
        })
    }
}
