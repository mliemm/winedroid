use super::super::*;

impl DexProgram {
    pub(super) fn invoke_android_activity_window_builtin(
        &self,
        method: &DexMethodId,
        invocation: Option<&DexInvocation>,
    ) -> Option<DexExecutionResult> {
        if method.class_descriptor == "Landroid/app/Activity;"
            && method.name == "<init>"
            && method.proto.descriptor_is("()V")
        {
            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/app/Application;"
            && method.name == "<init>"
            && method.proto.descriptor_is("()V")
        {
            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/app/Application;"
            && method.name == "onCreate"
            && method.proto.descriptor_is("()V")
        {
            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/app/Application;"
            && method.name == "registerActivityLifecycleCallbacks"
            && method.proto.descriptor()
                == "(Landroid/app/Application$ActivityLifecycleCallbacks;)V"
        {
            let invocation = invocation?;
            self.register_application_activity_lifecycle_callback(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/view/WindowManager$LayoutParams;"
            && method.name == "<init>"
            && method.proto.descriptor_is("()V")
        {
            let invocation = invocation?;
            self.init_window_manager_layout_params(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/view/WindowManager$LayoutParams;"
            && method.name == "setTitle"
            && method.proto.descriptor_is("(Ljava/lang/CharSequence;)V")
        {
            let invocation = invocation?;
            self.window_manager_layout_params_set_title(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/view/WindowManager$LayoutParams;"
            && method.name == "getTitle"
            && method.proto.descriptor_is("()Ljava/lang/CharSequence;")
        {
            let invocation = invocation?;
            let value = self.window_manager_layout_params_title(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Landroid/app/Activity;"
            && method.name == "onCreateOptionsMenu"
            && method.proto.descriptor_is("(Landroid/view/Menu;)Z")
        {
            // cite: frameworks-base Activity.java#onCreateOptionsMenu
            return Some(DexExecutionResult::Int(1));
        }

        if method.class_descriptor == "Landroid/app/Activity;"
            && method.name == "invalidateOptionsMenu"
            && method.proto.descriptor_is("()V")
        {
            let invocation = invocation?;
            self.post_invalidate_options_menu(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/app/Activity;"
            && method.name == "getMenuInflater"
            && method.proto.descriptor_is("()Landroid/view/MenuInflater;")
        {
            return Some(DexExecutionResult::Value(
                self.allocate_object("Landroid/view/MenuInflater;".to_owned()),
            ));
        }

        if matches!(
            method.class_descriptor.as_str(),
            "Landroid/app/Activity;" | "Landroidx/appcompat/app/AppCompatActivity;"
        ) && matches!(method.name.as_str(), "getActionBar" | "getSupportActionBar")
            && matches!(
                method.proto.descriptor().as_str(),
                "()Landroid/app/ActionBar;" | "()Landroidx/appcompat/app/ActionBar;"
            )
        {
            self.runtime.borrow_mut().action_bar_present = true;
            if self.runtime.borrow().action_bar_title.is_none() {
                if let Some(class_descriptor) = invocation
                    .and_then(|invocation| invocation.arguments.first())
                    .and_then(value_class_descriptor)
                    .map(str::to_owned)
                {
                    match self.resolve_activity_title(&class_descriptor) {
                        Ok(Some(title)) => {
                            self.runtime.borrow_mut().action_bar_title = Some(title);
                        }
                        Ok(None) => {}
                        Err(error) => {
                            return Some(DexExecutionResult::Throw(
                                self.resource_not_found_exception(error),
                            ));
                        }
                    }
                }
            }
            return Some(DexExecutionResult::Value(DexValue::Reference(
                std::rc::Rc::from("Landroid/app/ActionBar;.instance"),
            )));
        }

        if matches!(
            method.class_descriptor.as_str(),
            "Landroid/app/ActionBar;" | "Landroidx/appcompat/app/ActionBar;"
        ) {
            if method.name == "setDisplayHomeAsUpEnabled" && method.proto.descriptor_is("(Z)V") {
                let enabled = matches!(
                    invocation.and_then(|invocation| invocation.arguments.get(1)),
                    Some(DexValue::Int(value)) if *value != 0
                );
                self.runtime.borrow_mut().action_bar_home_as_up = enabled;
                return Some(DexExecutionResult::Void);
            }

            if matches!(
                method.name.as_str(),
                "setHomeButtonEnabled"
                    | "setDisplayShowHomeEnabled"
                    | "setDisplayShowTitleEnabled"
                    | "setDisplayUseLogoEnabled"
                    | "show"
                    | "hide"
            ) && matches!(method.proto.descriptor().as_str(), "(Z)V" | "()V")
            {
                self.runtime.borrow_mut().action_bar_present = true;
                return Some(DexExecutionResult::Void);
            }

            if method.name == "setTitle"
                && matches!(
                    method.proto.descriptor().as_str(),
                    "(Ljava/lang/CharSequence;)V" | "(I)V"
                )
            {
                let Some(value) = invocation.and_then(|invocation| invocation.arguments.get(1))
                else {
                    return Some(DexExecutionResult::Void);
                };
                let title = match self.action_bar_title_argument(value) {
                    Ok(title) => title,
                    Err(error) => {
                        return Some(DexExecutionResult::Throw(
                            self.resource_not_found_exception(error),
                        ));
                    }
                };
                if let Some(title) = title {
                    let mut runtime = self.runtime.borrow_mut();
                    runtime.action_bar_present = true;
                    runtime.action_bar_title = Some(title);
                }
                return Some(DexExecutionResult::Void);
            }

            if method.name == "setSubtitle"
                && matches!(
                    method.proto.descriptor().as_str(),
                    "(Ljava/lang/CharSequence;)V" | "(I)V"
                )
            {
                return Some(DexExecutionResult::Void);
            }
        }

        if method.class_descriptor == "Landroid/app/Activity;"
            && method.name == "onCreate"
            && method.proto.descriptor_is("(Landroid/os/Bundle;)V")
        {
            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/app/Activity;"
            && matches!(method.name.as_str(), "onStart" | "onResume")
            && method.proto.descriptor_is("()V")
        {
            return Some(DexExecutionResult::Void);
        }

        // cite: frameworks-base Dialog.java#onCreate, onStart
        if method.class_descriptor == "Landroid/app/Dialog;"
            && matches!(
                (method.name.as_str(), method.proto.descriptor().as_str()),
                ("onCreate", "(Landroid/os/Bundle;)V") | ("onStart", "()V") | ("onStop", "()V")
            )
        {
            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/app/Dialog;"
            && method.name == "<init>"
            && method.proto.descriptor_is("(Landroid/content/Context;I)V")
        {
            let invocation = invocation?;
            self.dialog_init(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        // cite: frameworks-base Dialog.java#Dialog
        if method.class_descriptor == "Landroid/app/Dialog;"
            && method.name == "<init>"
            && method.proto.descriptor_is("(Landroid/content/Context;)V")
        {
            let invocation = invocation?;
            if invocation.arguments.len() != 2 {
                return None;
            }
            let themed_invocation = DexInvocation {
                arguments: smallvec::smallvec![
                    invocation.arguments[0].clone(),
                    invocation.arguments[1].clone(),
                    DexValue::Int(0),
                ],
                ..invocation.clone()
            };
            self.dialog_init(&themed_invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/app/Dialog;"
            && method.name == "getContext"
            && method.proto.descriptor_is("()Landroid/content/Context;")
        {
            let invocation = invocation?;
            let value = self.dialog_context(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Landroid/app/Dialog;"
            && method.name == "getWindow"
            && method.proto.descriptor_is("()Landroid/view/Window;")
        {
            let invocation = invocation?;
            let value = self.dialog_window(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        // cite: frameworks-base Dialog.java#setTitle, requestWindowFeature, setOwnerActivity, create
        if method.class_descriptor == "Landroid/app/Dialog;"
            && matches!(
                (method.name.as_str(), method.proto.descriptor().as_str()),
                ("setTitle", "(Ljava/lang/CharSequence;)V")
                    | ("setTitle", "(I)V")
                    | ("setOwnerActivity", "(Landroid/app/Activity;)V")
                    | ("create", "()V")
                    | ("setVolumeControlStream", "(I)V")
            )
        {
            return Some(DexExecutionResult::Void);
        }

        // cite: frameworks-base Dialog.java#setContentView
        if method.class_descriptor == "Landroid/app/Dialog;"
            && matches!(
                (method.name.as_str(), method.proto.descriptor().as_str()),
                ("setContentView", "(Landroid/view/View;)V")
                    | (
                        "setContentView",
                        "(Landroid/view/View;Landroid/view/ViewGroup$LayoutParams;)V"
                    )
            )
        {
            let invocation = invocation?;
            self.write_dialog_content_view(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        // cite: frameworks-base Dialog.java#setContentView
        if method.class_descriptor == "Landroid/app/Dialog;"
            && method.name == "setContentView"
            && method.proto.descriptor_is("(I)V")
        {
            let invocation = invocation?;
            self.dialog_set_content_view_by_resource(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        // cite: frameworks-base Dialog.java#findViewById
        if method.class_descriptor == "Landroid/app/Dialog;"
            && method.name == "findViewById"
            && method.proto.descriptor_is("(I)Landroid/view/View;")
        {
            let invocation = invocation?;
            let value = self.view_find_view_by_id(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Landroid/app/Dialog;"
            && method.name == "requestWindowFeature"
            && method.proto.descriptor_is("(I)Z")
        {
            // cite: frameworks-base Dialog.java#requestWindowFeature
            return Some(DexExecutionResult::Int(1));
        }

        if method.class_descriptor == "Landroid/app/Dialog;"
            && method.name == "setCancelable"
            && method.proto.descriptor_is("(Z)V")
        {
            let invocation = invocation?;
            self.write_dialog_cancelable(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/app/Dialog;"
            && method.name == "setCanceledOnTouchOutside"
            && method.proto.descriptor_is("(Z)V")
        {
            let invocation = invocation?;
            self.write_dialog_canceled_on_touch_outside(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/app/Dialog;"
            && method.name == "setOnCancelListener"
            && method
                .proto
                .descriptor_is("(Landroid/content/DialogInterface$OnCancelListener;)V")
        {
            let invocation = invocation?;
            self.write_dialog_on_cancel_listener(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/app/Dialog;"
            && method.name == "setOnDismissListener"
            && method
                .proto
                .descriptor_is("(Landroid/content/DialogInterface$OnDismissListener;)V")
        {
            let invocation = invocation?;
            self.write_dialog_on_dismiss_listener(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/app/Dialog;"
            && method.name == "show"
            && method.proto.descriptor_is("()V")
        {
            let invocation = invocation?;
            self.dialog_show(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/app/Dialog;"
            && method.name == "isShowing"
            && method.proto.descriptor_is("()Z")
        {
            let invocation = invocation?;
            let value = self.dialog_is_showing(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        // cite: frameworks-base Context.java#setTheme
        // cite: frameworks-base Activity.java#setTheme
        if matches!(
            method.class_descriptor.as_str(),
            "Landroid/app/Activity;" | "Landroid/content/Context;"
        ) && method.name == "setTheme"
            && method.proto.descriptor_is("(I)V")
        {
            let invocation = invocation?;
            self.activity_set_theme(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        // cite: frameworks-base Activity.java#setRequestedOrientation
        if method.class_descriptor == "Landroid/app/Activity;"
            && method.name == "setRequestedOrientation"
            && method.proto.descriptor_is("(I)V")
        {
            let invocation = invocation?;
            self.activity_write_requested_orientation(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        // cite: frameworks-base Activity.java#setShowWhenLocked, setTurnScreenOn
        if method.class_descriptor == "Landroid/app/Activity;"
            && matches!(
                method.name.as_str(),
                "setShowWhenLocked" | "setTurnScreenOn"
            )
            && method.proto.descriptor_is("(Z)V")
        {
            let invocation = invocation?;
            self.activity_write_window_bool(invocation, &method.name)?;

            return Some(DexExecutionResult::Void);
        }

        // cite: frameworks-base Activity.java#getRequestedOrientation
        if method.class_descriptor == "Landroid/app/Activity;"
            && method.name == "getRequestedOrientation"
            && method.proto.descriptor_is("()I")
        {
            let invocation = invocation?;
            let value = self.activity_read_requested_orientation(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Landroid/app/Activity;"
            && method.name == "registerActivityLifecycleCallbacks"
            && method.proto.descriptor()
                == "(Landroid/app/Application$ActivityLifecycleCallbacks;)V"
        {
            let invocation = invocation?;
            self.register_activity_lifecycle_callback(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/app/Activity;"
            && method.name == "getFragmentManager"
            && method
                .proto
                .descriptor_is("()Landroid/app/FragmentManager;")
        {
            return Some(DexExecutionResult::Value(DexValue::Reference(
                std::rc::Rc::from("Landroid/app/FragmentManager;"),
            )));
        }

        if method.class_descriptor == "Landroid/app/Activity;"
            && method.name == "getWindow"
            && method.proto.descriptor_is("()Landroid/view/Window;")
        {
            let invocation = invocation?;
            let value = self.activity_window(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Landroid/app/Activity;"
            && method.name == "setContentView"
            && method.proto.descriptor_is("(Landroid/view/View;)V")
        {
            let invocation = invocation?;
            self.activity_set_content_view(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/app/Activity;"
            && method.name == "setContentView"
            && method.proto.descriptor_is("(I)V")
        {
            let invocation = invocation?;
            self.record_content_view_argument(invocation, 1);

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/app/Activity;"
            && method.name == "findViewById"
            && method.proto.descriptor_is("(I)Landroid/view/View;")
        {
            let invocation = invocation?;
            let value = self.view_find_view_by_id(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Landroid/app/Activity;"
            && method.name == "setContentView"
            && method.proto.descriptor()
                == "(Landroid/view/View;Landroid/view/ViewGroup$LayoutParams;)V"
        {
            let invocation = invocation?;
            self.activity_set_content_view(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/app/Activity;"
            && method.name == "getLayoutInflater"
            && method
                .proto
                .descriptor_is("()Landroid/view/LayoutInflater;")
        {
            // cite: frameworks-base Activity.java#getLayoutInflater
            let activity = invocation?.arguments.first()?.clone();
            return Some(DexExecutionResult::Value(
                self.layout_inflater_for_context(&activity),
            ));
        }

        // cite: frameworks-base Activity.java#getIntent, setIntent, mIntent
        let activity_intent_field = || DexFieldId {
            class_descriptor: "Landroid/app/Activity;".to_owned(),
            type_descriptor: "Landroid/content/Intent;".to_owned(),
            name: "mIntent".to_owned(),
        };
        if method.matches_builtin_signature(
            "Landroid/app/Activity;",
            "getIntent",
            "()Landroid/content/Intent;",
        ) && !self.guest_overrides(method, invocation)
        {
            let stored = invocation
                .and_then(|invocation| invocation.arguments.first())
                .and_then(|activity| {
                    self.read_stored_instance_field(activity, &activity_intent_field())
                });

            return Some(DexExecutionResult::Value(stored.unwrap_or_else(|| {
                DexValue::Reference(std::rc::Rc::from("Landroid/content/Intent;.bootstrap"))
            })));
        }

        if method.matches_builtin_signature(
            "Landroid/app/Activity;",
            "getReferrer",
            "()Landroid/net/Uri;",
        ) && !self.guest_overrides(method, invocation)
        {
            // cite: frameworks-base Activity.java#getReferrer, mReferrer
            // cite: frameworks-base Intent.java#EXTRA_REFERRER, EXTRA_REFERRER_NAME
            let invocation = invocation?;
            let activity = invocation.arguments.first()?;
            let parse = |uri: String| {
                self.parse_uri(&DexInvocation {
                    arguments: smallvec::smallvec![DexValue::String(std::rc::Rc::from(uri))],
                    ..invocation.clone()
                })
            };
            if let Some(intent) =
                self.read_stored_instance_field(activity, &activity_intent_field())
            {
                let extra = |name: &str| {
                    self.runtime
                        .borrow()
                        .intent_extras
                        .get(&(map_identity_key(&intent), name.to_owned()))
                        .cloned()
                };
                if let Some(referrer) =
                    extra("android.intent.extra.REFERRER").filter(dex_value_is_object_like)
                {
                    return Some(DexExecutionResult::Value(referrer));
                }
                if let Some(DexValue::String(name)) = extra("android.intent.extra.REFERRER_NAME") {
                    return Some(DexExecutionResult::Value(parse(name.to_string())?));
                }
            }
            let root = self
                .runtime
                .borrow()
                .activity_stack
                .first()
                .is_none_or(|root| map_identity_key(&root.receiver) == map_identity_key(activity));
            let referrer = if root {
                "com.android.shell".to_owned()
            } else {
                self.package_name.clone().unwrap_or_default()
            };

            return Some(DexExecutionResult::Value(parse(format!(
                "android-app://{referrer}"
            ))?));
        }

        if method.matches_builtin_signature(
            "Landroid/app/Activity;",
            "setIntent",
            "(Landroid/content/Intent;)V",
        ) && !self.guest_overrides(method, invocation)
        {
            let [activity, intent] = invocation?.arguments.as_slice() else {
                return None;
            };
            if !dex_value_is_object_like(activity) || !dex_value_is_nullable_object_like(intent) {
                return None;
            }
            let intent = if dex_value_is_object_like(intent) {
                intent.clone()
            } else {
                DexValue::Null
            };
            self.write_stored_instance_field(activity, &activity_intent_field(), intent)
                .then_some(())?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/app/Activity;"
            && method.name == "getApplication"
            && method.proto.descriptor_is("()Landroid/app/Application;")
        {
            return Some(DexExecutionResult::Value(self.application_value()));
        }

        if method.class_descriptor == "Landroid/app/Activity;"
            && method.name == "getComponentName"
            && method
                .proto
                .descriptor_is("()Landroid/content/ComponentName;")
            && !self.guest_overrides(method, invocation)
        {
            // cite: frameworks-base Activity.java#getComponentName, mComponent
            let activity = invocation?.arguments.first()?;
            let class = descriptor_to_java_name(value_class_descriptor(activity)?)?;
            let package = self.package_name.clone().unwrap_or_default();
            return Some(DexExecutionResult::Value(
                self.new_component_name(&package, &class),
            ));
        }

        // cite: frameworks-base Dialog.java#getOnBackInvokedDispatcher
        // cite: frameworks-base Activity.java#getOnBackInvokedDispatcher
        if matches!(
            method.class_descriptor.as_str(),
            "Landroid/app/Activity;" | "Landroid/app/Dialog;"
        ) && method.name == "getOnBackInvokedDispatcher"
            && method
                .proto
                .descriptor_is("()Landroid/window/OnBackInvokedDispatcher;")
        {
            return Some(DexExecutionResult::Value(DexValue::Reference(
                std::rc::Rc::from("Landroid/window/OnBackInvokedDispatcher;.bootstrap"),
            )));
        }

        if method.class_descriptor == "Landroid/app/Activity;"
            && method.name == "getResources"
            && method
                .proto
                .descriptor_is("()Landroid/content/res/Resources;")
        {
            return Some(DexExecutionResult::Value(DexValue::Reference(
                std::rc::Rc::from("Landroid/content/res/Resources;.bootstrap"),
            )));
        }

        if method.class_descriptor == "Landroid/app/Activity;"
            && method.name == "getTitle"
            && method.proto.descriptor_is("()Ljava/lang/CharSequence;")
        {
            let class_descriptor = invocation
                .and_then(|invocation| invocation.arguments.first())
                .and_then(value_class_descriptor);
            let title = match class_descriptor {
                Some(class_descriptor) => match self.resolve_activity_title(class_descriptor) {
                    Ok(title) => title.unwrap_or_default(),
                    Err(error) => {
                        return Some(DexExecutionResult::Throw(
                            self.resource_not_found_exception(error),
                        ));
                    }
                },
                None => String::new(),
            };
            return Some(DexExecutionResult::Value(DexValue::String(
                std::rc::Rc::from(title),
            )));
        }

        if method.class_descriptor == "Landroid/app/Activity;"
            && method.name == "getLastNonConfigurationInstance"
            && method.proto.descriptor_is("()Ljava/lang/Object;")
        {
            return Some(DexExecutionResult::Value(DexValue::Null));
        }

        if method.matches_builtin_signature("Landroid/app/Activity;", "hasWindowFocus", "()Z")
            && !self.guest_overrides(method, invocation)
        {
            // cite: frameworks-base Activity.java#hasWindowFocus
            let activity = invocation?.arguments.first()?;
            let focused = self
                .runtime
                .borrow()
                .window_focused_activities
                .contains(&map_identity_key(activity));
            return Some(DexExecutionResult::Int(i32::from(focused)));
        }

        if method.class_descriptor == "Landroid/app/Activity;"
            && matches!(
                method.name.as_str(),
                "isInMultiWindowMode" | "isInPictureInPictureMode"
            )
            && method.proto.descriptor_is("()Z")
            && !self.guest_overrides(method, invocation)
        {
            // cite: frameworks-base Activity.java#isInMultiWindowMode, isInPictureInPictureMode, mIsInMultiWindowMode, mIsInPictureInPictureMode
            return Some(DexExecutionResult::Int(0));
        }

        if method.class_descriptor == "Landroid/app/Activity;"
            && matches!(method.name.as_str(), "isFinishing" | "isDestroyed")
            && method.proto.descriptor_is("()Z")
            && !self.guest_overrides(method, invocation)
        {
            // cite: frameworks-base Activity.java#isFinishing, isDestroyed, mFinished, mDestroyed
            let state = invocation
                .and_then(|invocation| invocation.arguments.first())
                .is_some_and(|activity| {
                    let runtime = self.runtime.borrow();
                    let key = map_identity_key(activity);
                    if method.name == "isFinishing" {
                        runtime.finished_activities.contains(&key)
                    } else {
                        runtime.destroyed_activities.contains(&key)
                    }
                });
            return Some(DexExecutionResult::Int(i32::from(state)));
        }

        if method.class_descriptor == "Landroid/app/Application;"
            && method.name == "getProcessName"
            && method.proto.descriptor_is("()Ljava/lang/String;")
        {
            return Some(DexExecutionResult::Value(DexValue::String(
                std::rc::Rc::from(self.bootstrap_process_name()),
            )));
        }

        if method.class_descriptor == "Landroid/view/Window;"
            && method.name == "requestFeature"
            && method.proto.descriptor_is("(I)Z")
        {
            // cite: frameworks-base PhoneWindow.java#requestFeature
            let invocation = invocation?;
            let DexValue::Int(feature_id) = *invocation.arguments.get(1)? else {
                return None;
            };
            self.write_stored_instance_field(
                &invocation.arguments[0],
                &window_manager_layout_params_int_field("features"),
                DexValue::Int(1 << feature_id),
            );

            return Some(DexExecutionResult::Int(1));
        }

        if method.class_descriptor == "Landroid/view/Window;"
            && method.name == "getDecorView"
            && method.proto.descriptor_is("()Landroid/view/View;")
        {
            // cite: frameworks-base PhoneWindow.java#getDecorView, installDecor, generateDecor
            return Some(DexExecutionResult::Value(decor_view_reference()));
        }

        if method.class_descriptor == "Landroid/view/Window;"
            && method.name == "peekDecorView"
            && method.proto.descriptor_is("()Landroid/view/View;")
        {
            // cite: frameworks-base PhoneWindow.java#peekDecorView
            return Some(DexExecutionResult::Value(decor_view_reference()));
        }

        if method.class_descriptor == "Landroid/view/Window;"
            && method.name == "getCallback"
            && method
                .proto
                .descriptor_is("()Landroid/view/Window$Callback;")
        {
            let invocation = invocation?;
            let value = self.read_window_callback(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Landroid/view/Window;"
            && method.name == "getAttributes"
            && method
                .proto
                .descriptor_is("()Landroid/view/WindowManager$LayoutParams;")
        {
            let invocation = invocation?;
            let value = self.window_layout_params(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Landroid/view/Window;"
            && method.name == "setAttributes"
            && method
                .proto
                .descriptor_is("(Landroid/view/WindowManager$LayoutParams;)V")
        {
            let invocation = invocation?;
            self.write_window_layout_params(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/view/Window;"
            && method.name == "setCallback"
            && method
                .proto
                .descriptor_is("(Landroid/view/Window$Callback;)V")
        {
            let invocation = invocation?;
            self.write_window_callback(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/view/Window;"
            && method.name == "setDecorFitsSystemWindows"
            && method.proto.descriptor_is("(Z)V")
        {
            let invocation = invocation?;
            self.write_window_boolean(invocation, &window_decor_fits_system_windows_field())?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/view/Window;"
            && matches!(
                method.name.as_str(),
                "setStatusBarContrastEnforced" | "setNavigationBarContrastEnforced"
            )
            && method.proto.descriptor_is("(Z)V")
        {
            let invocation = invocation?;
            self.write_window_boolean(invocation, &window_contrast_enforced_field(&method.name))?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/view/Window;"
            && matches!(
                method.name.as_str(),
                "isStatusBarContrastEnforced" | "isNavigationBarContrastEnforced"
            )
            && method.proto.descriptor_is("()Z")
        {
            let invocation = invocation?;
            let value = self
                .read_window_boolean(invocation, &window_contrast_enforced_field(&method.name))?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Landroid/view/Window$Callback;"
            && method.name == "onContentChanged"
            && method.proto.descriptor_is("()V")
        {
            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/view/Window$Callback;"
            && method.name == "onCreatePanelView"
            && method.proto.descriptor_is("(I)Landroid/view/View;")
        {
            return Some(DexExecutionResult::Value(DexValue::Null));
        }

        if method.class_descriptor == "Landroid/view/Window$Callback;"
            && method.name == "onCreatePanelMenu"
            && method.proto.descriptor_is("(ILandroid/view/Menu;)Z")
        {
            let invocation = invocation?;
            if invocation.arguments.len() != 3
                || !dex_value_is_object_like(&invocation.arguments[0])
                || !dex_value_is_object_like(&invocation.arguments[2])
            {
                return None;
            }

            return Some(DexExecutionResult::Int(1));
        }

        if method.class_descriptor == "Landroid/view/Window$Callback;"
            && method.name == "onPreparePanel"
            && method
                .proto
                .descriptor_is("(ILandroid/view/View;Landroid/view/Menu;)Z")
        {
            let invocation = invocation?;
            if invocation.arguments.len() != 4
                || !dex_value_is_object_like(&invocation.arguments[0])
                || !dex_value_is_nullable_object_like(&invocation.arguments[2])
                || !dex_value_is_object_like(&invocation.arguments[3])
            {
                return None;
            }

            return Some(DexExecutionResult::Int(1));
        }

        if method.class_descriptor == "Landroid/view/Window;"
            && method.name == "addFlags"
            && method.proto.descriptor_is("(I)V")
        {
            let invocation = invocation?;
            window_add_flags(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        // cite: frameworks-base Window.java#setBackgroundDrawableResource, setLayout, setGravity, setDimAmount, setWindowAnimations, setSoftInputMode, clearFlags
        // cite: frameworks-base PhoneWindow.java#setElevation
        if method.class_descriptor == "Landroid/view/Window;"
            && matches!(
                (method.name.as_str(), method.proto.descriptor().as_str()),
                ("setBackgroundDrawableResource", "(I)V")
                    | (
                        "setBackgroundDrawable",
                        "(Landroid/graphics/drawable/Drawable;)V"
                    )
                    | ("setBackgroundBlurRadius", "(I)V")
                    | ("setLayout", "(II)V")
                    | ("setGravity", "(I)V")
                    | ("setDimAmount", "(F)V")
                    | ("setWindowAnimations", "(I)V")
                    | ("setSoftInputMode", "(I)V")
                    | ("clearFlags", "(I)V")
                    | ("setElevation", "(F)V")
                    | ("setFlags", "(II)V")
                    | ("setType", "(I)V")
            )
        {
            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/view/Window;"
            && matches!(
                method.name.as_str(),
                "setStatusBarColor" | "setNavigationBarColor"
            )
            && method.proto.descriptor_is("(I)V")
        {
            let invocation = invocation?;
            self.write_window_color(invocation, &window_bar_color_field(&method.name))?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/view/Window;"
            && matches!(
                method.name.as_str(),
                "getStatusBarColor" | "getNavigationBarColor"
            )
            && method.proto.descriptor_is("()I")
        {
            let invocation = invocation?;
            let value =
                self.read_window_color(invocation, &window_bar_color_field(&method.name))?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Landroid/view/Window;"
            && method.name == "findViewById"
            && method.proto.descriptor_is("(I)Landroid/view/View;")
        {
            let invocation = invocation?;
            let value = self.view_find_view_by_id(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Landroid/view/Window;"
            && method.name == "getInsetsController"
            && method
                .proto
                .descriptor_is("()Landroid/view/WindowInsetsController;")
        {
            let invocation = invocation?;
            let value = self.window_insets_controller(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Landroid/view/WindowInsetsController;"
            && method.name == "setSystemBarsAppearance"
            && method.proto.descriptor_is("(II)V")
        {
            let invocation = invocation?;
            self.write_window_insets_controller_system_bars_appearance(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/view/WindowInsetsController;"
            && method.name == "getSystemBarsAppearance"
            && method.proto.descriptor_is("()I")
        {
            let invocation = invocation?;
            let value = self.read_window_insets_controller_system_bars_appearance(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Landroid/view/WindowInsetsController;"
            && method.name == "show"
            && method.proto.descriptor_is("(I)V")
        {
            let invocation = invocation?;
            window_insets_controller_visibility_request(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        // cite: frameworks-base WindowInsetsController.java#addOnControllableInsetsChangedListener, removeOnControllableInsetsChangedListener, hide, setSystemBarsBehavior
        if method.class_descriptor == "Landroid/view/WindowInsetsController;"
            && matches!(
                (method.name.as_str(), method.proto.descriptor().as_str()),
                (
                    "addOnControllableInsetsChangedListener",
                    "(Landroid/view/WindowInsetsController$OnControllableInsetsChangedListener;)V"
                ) | (
                    "removeOnControllableInsetsChangedListener",
                    "(Landroid/view/WindowInsetsController$OnControllableInsetsChangedListener;)V"
                ) | ("hide", "(I)V")
                    | ("setSystemBarsBehavior", "(I)V")
            )
        {
            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/view/Window;"
            && method.name == "setContentView"
            && method.proto.descriptor_is("(Landroid/view/View;)V")
        {
            let invocation = invocation?;
            // cite: frameworks-base PhoneWindow.java#setContentView
            self.activity_set_content_view(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/view/WindowManager;"
            && method.name == "getDefaultDisplay"
            && method.proto.descriptor_is("()Landroid/view/Display;")
        {
            return Some(DexExecutionResult::Value(DexValue::Reference(
                std::rc::Rc::from("Landroid/view/Display;.bootstrap"),
            )));
        }

        if method.matches_builtin_signature(
            "Landroid/view/WindowManager;",
            "isCrossWindowBlurEnabled",
            "()Z",
        ) {
            // cite: frameworks-base WindowManager.java#isCrossWindowBlurEnabled
            return Some(DexExecutionResult::Int(0));
        }

        if method.class_descriptor == "Landroid/view/WindowManager;"
            && method.name == "getCurrentWindowMetrics"
            && method.proto.descriptor_is("()Landroid/view/WindowMetrics;")
        {
            let invocation = invocation?;
            let value = self.current_window_metrics(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Landroid/view/WindowMetrics;"
            && method.name == "getBounds"
            && method.proto.descriptor_is("()Landroid/graphics/Rect;")
        {
            let invocation = invocation?;
            let value = self.window_metrics_bounds(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Landroid/view/Display;"
            && matches!(method.name.as_str(), "getSize" | "getRealSize")
            && method.proto.descriptor_is("(Landroid/graphics/Point;)V")
        {
            let invocation = invocation?;
            self.display_write_size(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/view/Display;"
            && matches!(method.name.as_str(), "getMetrics" | "getRealMetrics")
            && method
                .proto
                .descriptor_is("(Landroid/util/DisplayMetrics;)V")
        {
            let invocation = invocation?;
            self.display_write_metrics(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/view/Display;"
            && method.name == "getWidth"
            && method.proto.descriptor_is("()I")
        {
            // cite: frameworks-base Display.java#getWidth
            return Some(DexExecutionResult::Int(self.display_size().width_i32()));
        }

        if method.class_descriptor == "Landroid/view/Display;"
            && method.name == "getHeight"
            && method.proto.descriptor_is("()I")
        {
            // cite: frameworks-base Display.java#getHeight
            return Some(DexExecutionResult::Int(self.display_size().height_i32()));
        }

        if method.class_descriptor == "Landroid/view/Display;"
            && method.name == "getRotation"
            && method.proto.descriptor_is("()I")
        {
            return Some(DexExecutionResult::Int(0));
        }

        if method.class_descriptor == "Landroid/view/Display;"
            && method.name == "getRefreshRate"
            && method.proto.descriptor_is("()F")
        {
            return Some(DexExecutionResult::Value(DexValue::Float(60.0)));
        }

        if method.class_descriptor == "Landroid/graphics/Point;"
            && method.name == "<init>"
            && method.proto.descriptor_is("()V")
        {
            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/graphics/Insets;"
            && method.name == "of"
            && method
                .proto
                .descriptor_is("(IIII)Landroid/graphics/Insets;")
        {
            let invocation = invocation?;
            let value = self.android_graphics_insets_of(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // cite: frameworks-base Display.java#getWidth, getHeight
    // cite: frameworks-base WindowMetricsController.java#getCurrentWindowMetrics, getWindowMetricsInternal
    // cite: frameworks-base WindowMetrics.java#getBounds
    #[test]
    fn display_and_window_metrics_share_configured_non_default_dimensions() {
        let config = WinedroidConfig::with_display_size(600, 1000);
        let program = DexProgram::with_config(Vec::new(), config);
        let get_width = display_int_getter("getWidth");
        let get_height = display_int_getter("getHeight");

        assert_eq!(
            program.invoke_builtin_method(&get_width, None),
            Some(DexExecutionResult::Int(600))
        );
        assert_eq!(
            program.invoke_builtin_method(&get_height, None),
            Some(DexExecutionResult::Int(1000))
        );

        let metrics = DexValue::Reference(std::rc::Rc::from(
            "Landroid/view/WindowMetrics;:configured-display-test",
        ));
        let bounds = DexValue::Reference(std::rc::Rc::from(
            "Landroid/graphics/Rect;:configured-display-test",
        ));
        program
            .write_window_metrics_bounds(&metrics, &bounds)
            .expect("window metrics bounds should be written");

        for (name, expected) in [("left", 0), ("top", 0), ("right", 600), ("bottom", 1000)] {
            assert_eq!(
                program.read_stored_instance_field(&bounds, &rect_int_field(name)),
                Some(DexValue::Int(expected))
            );
        }
    }

    fn display_int_getter(name: &str) -> DexMethodId {
        DexMethodId {
            class_descriptor: "Landroid/view/Display;".to_owned(),
            name: name.to_owned(),
            proto: crate::dex::DexProto {
                shorty: "I".to_owned(),
                return_type: "I".to_owned(),
                parameters: Vec::new(),
            },
        }
    }
}

fn window_insets_controller_visibility_request(invocation: &DexInvocation) -> Option<()> {
    if invocation.arguments.len() != 2 || !dex_value_is_object_like(&invocation.arguments[0]) {
        return None;
    }

    let DexValue::Int(_) = invocation.arguments[1] else {
        return None;
    };

    Some(())
}

impl DexProgram {
    fn current_window_metrics(&self, invocation: &DexInvocation) -> Option<DexValue> {
        if invocation.arguments.len() != 1 || !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let metrics =
            DexValue::Reference(std::rc::Rc::from("Landroid/view/WindowMetrics;:current"));
        let bounds = DexValue::Reference(std::rc::Rc::from(
            "Landroid/graphics/Rect;:window-metrics:current",
        ));
        self.write_window_metrics_bounds(&metrics, &bounds)?;

        Some(metrics)
    }

    fn window_metrics_bounds(&self, invocation: &DexInvocation) -> Option<DexValue> {
        if invocation.arguments.len() != 1 || !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        self.read_stored_instance_field(&invocation.arguments[0], &window_metrics_bounds_field())
    }

    fn write_window_metrics_bounds(&self, metrics: &DexValue, bounds: &DexValue) -> Option<()> {
        let display_size = self.display_size();
        // cite: frameworks-base WindowMetricsController.java#getCurrentWindowMetrics, getWindowMetricsInternal
        // cite: frameworks-base WindowMetrics.java#getBounds
        for (name, value) in [
            ("left", 0),
            ("top", 0),
            ("right", display_size.width_i32()),
            ("bottom", display_size.height_i32()),
        ] {
            if !self.write_stored_instance_field(
                bounds,
                &rect_int_field(name),
                DexValue::Int(value),
            ) {
                return None;
            }
        }

        self.write_stored_instance_field(metrics, &window_metrics_bounds_field(), bounds.clone())
            .then_some(())
    }

    fn action_bar_title_argument(&self, value: &DexValue) -> Result<Option<String>> {
        match value {
            DexValue::String(text) => Ok(Some(text.clone().to_string())),
            DexValue::Int(resource_id) => {
                let resource_id = u32::try_from(*resource_id).map_err(|error| {
                    WinedroidError::with_source(
                        format!("ActionBar title resource ID is negative: {resource_id}"),
                        error,
                    )
                })?;
                let title = self
                    .resolve_string_resource_text(resource_id)?
                    .ok_or_else(|| {
                        WinedroidError::new(format!(
                            "ActionBar title string resource 0x{resource_id:08x} was not found"
                        ))
                    })?;
                Ok(Some(title))
            }
            DexValue::Object(_) | DexValue::Reference(_) => Ok(self.char_sequence_text(value)),
            DexValue::Null | DexValue::Uninitialized | DexValue::Long(_) | DexValue::Float(_) => {
                Ok(None)
            }
        }
    }
}
