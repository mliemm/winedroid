use super::super::*;
use super::helpers::*;
use crate::dex_run::android::app::application_callbacks_run_first;

#[test]
fn builtin_application_constructor_and_on_create_return_void() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Landroid/app/Application;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let on_create = DexMethodId {
        class_descriptor: "Landroid/app/Application;".to_owned(),
        name: "onCreate".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&constructor, None),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&on_create, None),
        Some(DexExecutionResult::Void)
    );
}

#[test]
fn builtin_activity_base_lifecycle_methods_return_void() {
    let program = DexProgram::new(Vec::new());
    let on_create = DexMethodId {
        class_descriptor: "Landroid/app/Activity;".to_owned(),
        name: "onCreate".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/os/Bundle;".to_owned()],
        },
    };
    let on_start = DexMethodId {
        class_descriptor: "Landroid/app/Activity;".to_owned(),
        name: "onStart".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let on_resume = DexMethodId {
        class_descriptor: "Landroid/app/Activity;".to_owned(),
        name: "onResume".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&on_create, Some(&test_invocation(vec![DexValue::Null]))),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&on_start, None),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&on_resume, None),
        Some(DexExecutionResult::Void)
    );
}

#[test]
fn builtin_activity_lifecycle_callback_registration_returns_void() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/app/Activity;".to_owned(),
        name: "registerActivityLifecycleCallbacks".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/app/Application$ActivityLifecycleCallbacks;".to_owned()],
        },
    };
    let activity = DexValue::Reference(std::rc::Rc::from("Landroid/app/Activity;:test"));
    let callback = DexValue::Reference(std::rc::Rc::from(
        "Landroid/app/Application$ActivityLifecycleCallbacks;:test",
    ));

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![activity.clone(), callback.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .activity_lifecycle_callbacks
            .get(&map_identity_key(&activity))
            .cloned(),
        Some(vec![callback])
    );
}

#[test]
fn frameworks_base_application_registers_lifecycle_callbacks_without_dedupe() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/app/Application;".to_owned(),
        name: "registerActivityLifecycleCallbacks".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/app/Application$ActivityLifecycleCallbacks;".to_owned()],
        },
    };
    let application = program.application_value();
    let callback = DexValue::Reference(std::rc::Rc::from(
        "Landroid/app/Application$ActivityLifecycleCallbacks;:test",
    ));
    let invocation = test_invocation(vec![application.clone(), callback.clone()]);

    for _ in 0..2 {
        assert_eq!(
            program.invoke_builtin_method(&method, Some(&invocation)),
            Some(DexExecutionResult::Void)
        );
    }
    assert_eq!(
        program
            .runtime
            .borrow()
            .activity_lifecycle_callbacks
            .get(&map_identity_key(&application))
            .cloned(),
        Some(vec![callback.clone(), callback.clone()])
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![application, DexValue::Null]))
        ),
        None
    );
}

// cite: frameworks-base Application.java#registerActivityLifecycleCallbacks, unregisterActivityLifecycleCallbacks
// cite: libcore ArrayList.java#remove
#[test]
fn application_unregister_removes_one_matching_registration() {
    let program = DexProgram::new(Vec::new());
    let application = program.application_value();
    let make_callback = |name: &str| {
        DexValue::Reference(std::rc::Rc::from(format!(
            "Landroid/app/Application$ActivityLifecycleCallbacks;:{name}"
        )))
    };
    let first = make_callback("first");
    let second = make_callback("second");
    let register = |callback| {
        program
            .register_application_activity_lifecycle_callback(&test_invocation(vec![
                application.clone(),
                callback,
            ]))
            .unwrap();
    };
    let unregister = |callback| {
        program
            .unregister_application_activity_lifecycle_callback(
                &test_invocation(vec![application.clone(), callback]),
                0,
            )
            .unwrap()
            .unwrap();
    };
    register(first.clone());
    register(second.clone());
    register(first.clone());
    unregister(first.clone());
    unregister(make_callback("absent"));
    unregister(DexValue::Null);
    assert_eq!(
        program
            .runtime
            .borrow()
            .activity_lifecycle_callbacks
            .get(&map_identity_key(&application)),
        Some(&vec![second, first]),
    );
}

#[test]
fn frameworks_base_activity_dispatch_order_puts_application_callbacks_first_for_created_not_post_created(
) {
    assert!(application_callbacks_run_first("onActivityCreated"));
    assert!(application_callbacks_run_first("onActivityPreCreated"));
    assert!(application_callbacks_run_first("onActivityResumed"));
    assert!(!application_callbacks_run_first("onActivityPostCreated"));
    assert!(!application_callbacks_run_first("onActivityPaused"));
}

#[test]
fn builtin_activity_get_fragment_manager_returns_reference() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/app/Activity;".to_owned(),
        name: "getFragmentManager".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/app/FragmentManager;".to_owned(),
            parameters: Vec::new(),
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&method, None),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Landroid/app/FragmentManager;")
        )))
    );
}

#[test]
fn builtin_activity_get_window_returns_activity_scoped_window() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/app/Activity;".to_owned(),
        name: "getWindow".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/view/Window;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let activity = DexValue::Object(DexObjectRef {
        id: 1,
        class_descriptor: std::rc::Rc::from(
            "Lcom/darkempire78/opencalculator/activities/MainActivity;",
        ),
    });

    let result = program.invoke_builtin_method(&method, Some(&test_invocation(vec![activity])));
    let Some(DexExecutionResult::Value(DexValue::Reference(window))) = result else {
        panic!("Activity.getWindow should return a window reference");
    };
    assert!(window.starts_with("Landroid/view/Window;:activity:"));
}

#[test]
fn builtin_activity_get_layout_inflater_returns_bootstrap_inflater() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/app/Activity;".to_owned(),
        name: "getLayoutInflater".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/view/LayoutInflater;".to_owned(),
            parameters: Vec::new(),
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&method, None),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Landroid/view/LayoutInflater;.bootstrap")
        )))
    );
}

#[test]
fn builtin_activity_get_intent_returns_bootstrap_intent() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/app/Activity;".to_owned(),
        name: "getIntent".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/content/Intent;".to_owned(),
            parameters: Vec::new(),
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&method, None),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Landroid/content/Intent;.bootstrap")
        )))
    );
}

#[test]
fn builtin_activity_get_application_returns_bootstrap_application() {
    let program = DexProgram::new(Vec::new());
    let get_application = DexMethodId {
        class_descriptor: "Landroid/app/Activity;".to_owned(),
        name: "getApplication".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/app/Application;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get_component_name = DexMethodId {
        class_descriptor: "Landroid/app/Activity;".to_owned(),
        name: "getComponentName".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/content/ComponentName;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get_on_back_invoked_dispatcher = DexMethodId {
        class_descriptor: "Landroid/app/Activity;".to_owned(),
        name: "getOnBackInvokedDispatcher".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/window/OnBackInvokedDispatcher;".to_owned(),
            parameters: Vec::new(),
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&get_application, None),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Landroid/app/Application;.bootstrap")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&get_component_name, None),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Landroid/content/ComponentName;.bootstrap")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&get_on_back_invoked_dispatcher, None),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Landroid/window/OnBackInvokedDispatcher;.bootstrap")
        )))
    );
}

#[test]
fn builtin_activity_get_application_uses_manifest_application_class() {
    let program = DexProgram::with_application_class_descriptor(
        Vec::new(),
        Some("Lcom/example/App;".to_owned()),
    );
    let get_application = DexMethodId {
        class_descriptor: "Landroid/app/Activity;".to_owned(),
        name: "getApplication".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/app/Application;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get_class = DexMethodId {
        class_descriptor: "Ljava/lang/Object;".to_owned(),
        name: "getClass".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/Class;".to_owned(),
            parameters: Vec::new(),
        },
    };

    let Some(DexExecutionResult::Value(application)) =
        program.invoke_builtin_method(&get_application, None)
    else {
        panic!("getApplication should return an application reference");
    };

    assert_eq!(
        application,
        DexValue::Reference(std::rc::Rc::from("Lcom/example/App;.bootstrap"))
    );
    assert_eq!(
        program.invoke_builtin_method(&get_class, Some(&test_invocation(vec![application]))),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Ljava/lang/Class;:Lcom/example/App;")
        )))
    );
}

#[test]
fn builtin_activity_get_resources_returns_bootstrap_resources() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/app/Activity;".to_owned(),
        name: "getResources".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/content/res/Resources;".to_owned(),
            parameters: Vec::new(),
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&method, None),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Landroid/content/res/Resources;.bootstrap")
        )))
    );
}

#[test]
fn builtin_activity_get_title_resolves_label_not_package_name() {
    let program =
        DexProgram::with_package_name(Vec::new(), Some("com.example.calculator".to_owned()));
    let method = DexMethodId {
        class_descriptor: "Landroid/app/Activity;".to_owned(),
        name: "getTitle".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/CharSequence;".to_owned(),
            parameters: Vec::new(),
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&method, None),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from(String::new())
        )))
    );
}

#[test]
fn builtin_activity_get_last_non_configuration_instance_returns_null() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/app/Activity;".to_owned(),
        name: "getLastNonConfigurationInstance".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: Vec::new(),
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&method, None),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
}

#[test]
fn builtin_activity_constructor_returns_void() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/app/Activity;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&method, None),
        Some(DexExecutionResult::Void)
    );
}

// cite: frameworks-base Dialog.java#mContext
// cite: frameworks-base ContextWrapper.java#mBase
// cite: frameworks-base AlertController.java#mTitle, mButtonPositiveText
#[test]
fn builtin_dialog_constructor_stores_context_and_theme() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/app/Dialog;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VLI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/content/Context;".to_owned(), "I".to_owned()],
        },
    };
    let get_context = DexMethodId {
        class_descriptor: "Landroid/app/Dialog;".to_owned(),
        name: "getContext".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/content/Context;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get_window = DexMethodId {
        class_descriptor: "Landroid/app/Dialog;".to_owned(),
        name: "getWindow".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/view/Window;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set_cancelable = DexMethodId {
        class_descriptor: "Landroid/app/Dialog;".to_owned(),
        name: "setCancelable".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VZ".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Z".to_owned()],
        },
    };
    let set_canceled_on_touch_outside = DexMethodId {
        class_descriptor: "Landroid/app/Dialog;".to_owned(),
        name: "setCanceledOnTouchOutside".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VZ".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Z".to_owned()],
        },
    };
    let set_on_cancel_listener = DexMethodId {
        class_descriptor: "Landroid/app/Dialog;".to_owned(),
        name: "setOnCancelListener".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/content/DialogInterface$OnCancelListener;".to_owned()],
        },
    };
    let set_on_dismiss_listener = DexMethodId {
        class_descriptor: "Landroid/app/Dialog;".to_owned(),
        name: "setOnDismissListener".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/content/DialogInterface$OnDismissListener;".to_owned()],
        },
    };
    let show = DexMethodId {
        class_descriptor: "Landroid/app/Dialog;".to_owned(),
        name: "show".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let is_showing = DexMethodId {
        class_descriptor: "Landroid/app/Dialog;".to_owned(),
        name: "isShowing".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let dialog = DexValue::Object(DexObjectRef {
        id: 1,
        class_descriptor: std::rc::Rc::from("La/n;"),
    });
    let context = DexValue::Object(DexObjectRef {
        id: 3,
        class_descriptor: std::rc::Rc::from("Landroid/view/ContextThemeWrapper;"),
    });
    let activity = DexValue::Object(DexObjectRef {
        id: 4,
        class_descriptor: std::rc::Rc::from("Lexample/MainActivity;"),
    });
    let invocation = test_invocation(vec![
        dialog.clone(),
        context.clone(),
        DexValue::Int(16973835),
    ]);
    let context_field = DexFieldId {
        class_descriptor: "Landroid/app/Dialog;".to_owned(),
        type_descriptor: "Landroid/content/Context;".to_owned(),
        name: "mContext".to_owned(),
    };
    let theme_resource_field = DexFieldId {
        class_descriptor: "Landroid/app/Dialog;".to_owned(),
        type_descriptor: "I".to_owned(),
        name: "mThemeResource".to_owned(),
    };
    let cancelable_field = DexFieldId {
        class_descriptor: "Landroid/app/Dialog;".to_owned(),
        type_descriptor: "Z".to_owned(),
        name: "cancelable".to_owned(),
    };
    let canceled_on_touch_outside_field = DexFieldId {
        class_descriptor: "Landroid/app/Dialog;".to_owned(),
        type_descriptor: "Z".to_owned(),
        name: "canceledOnTouchOutside".to_owned(),
    };
    let cancel_listener_field = DexFieldId {
        class_descriptor: "Landroid/app/Dialog;".to_owned(),
        type_descriptor: "Landroid/content/DialogInterface$OnCancelListener;".to_owned(),
        name: "onCancelListener".to_owned(),
    };
    let dismiss_listener_field = DexFieldId {
        class_descriptor: "Landroid/app/Dialog;".to_owned(),
        type_descriptor: "Landroid/content/DialogInterface$OnDismissListener;".to_owned(),
        name: "onDismissListener".to_owned(),
    };
    let showing_field = DexFieldId {
        class_descriptor: "Landroid/app/Dialog;".to_owned(),
        type_descriptor: "Z".to_owned(),
        name: "showing".to_owned(),
    };
    let controller_field = DexFieldId {
        class_descriptor: "Lg/j;".to_owned(),
        type_descriptor: "Lg/i;".to_owned(),
        name: "f".to_owned(),
    };
    let controller_title_field = DexFieldId {
        class_descriptor: "Lg/i;".to_owned(),
        type_descriptor: "Ljava/lang/CharSequence;".to_owned(),
        name: "d".to_owned(),
    };
    let controller_button_field = DexFieldId {
        class_descriptor: "Lg/i;".to_owned(),
        type_descriptor: "Ljava/lang/CharSequence;".to_owned(),
        name: "g".to_owned(),
    };
    let controller_view_field = DexFieldId {
        class_descriptor: "Lg/i;".to_owned(),
        type_descriptor: "Landroid/view/View;".to_owned(),
        name: "o".to_owned(),
    };
    let context_base_field = DexFieldId {
        class_descriptor: "Landroid/content/ContextWrapper;".to_owned(),
        type_descriptor: "Landroid/content/Context;".to_owned(),
        name: "mBase".to_owned(),
    };
    let activity_history_field = DexFieldId {
        class_descriptor: "Lexample/MainActivity;".to_owned(),
        type_descriptor: "Ljava/lang/String;".to_owned(),
        name: "historyId".to_owned(),
    };
    let cancel_listener = DexValue::Reference(std::rc::Rc::from(
        "Landroid/content/DialogInterface$OnCancelListener;:test",
    ));
    let dismiss_listener = DexValue::Reference(std::rc::Rc::from(
        "Landroid/content/DialogInterface$OnDismissListener;:test",
    ));
    let controller = DexValue::Object(DexObjectRef {
        id: 2,
        class_descriptor: std::rc::Rc::from("Lg/i;"),
    });
    let dialog_content = DexValue::Reference(std::rc::Rc::from(
        "Landroidx/constraintlayout/widget/ConstraintLayout;:layout/2131492910",
    ));

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&dialog, &context_field),
        Some(context.clone())
    );
    assert_eq!(
        program.read_stored_instance_field(&dialog, &theme_resource_field),
        Some(DexValue::Int(16973835))
    );
    assert_eq!(
        program.invoke_builtin_method(&get_context, Some(&test_invocation(vec![dialog.clone()]))),
        Some(DexExecutionResult::Value(context.clone()))
    );
    let window_result =
        program.invoke_builtin_method(&get_window, Some(&test_invocation(vec![dialog.clone()])));
    let Some(DexExecutionResult::Value(DexValue::Reference(window))) = window_result else {
        panic!("Dialog.getWindow should return a window reference");
    };
    assert!(window.starts_with("Landroid/view/Window;:dialog:"));
    assert_eq!(
        program.invoke_builtin_method(
            &set_cancelable,
            Some(&test_invocation(vec![dialog.clone(), DexValue::Int(0)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&dialog, &cancelable_field),
        Some(DexValue::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_canceled_on_touch_outside,
            Some(&test_invocation(vec![dialog.clone(), DexValue::Int(1)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&dialog, &canceled_on_touch_outside_field),
        Some(DexValue::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_on_cancel_listener,
            Some(&test_invocation(vec![
                dialog.clone(),
                cancel_listener.clone()
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&dialog, &cancel_listener_field),
        Some(cancel_listener)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_on_dismiss_listener,
            Some(&test_invocation(vec![
                dialog.clone(),
                dismiss_listener.clone()
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&dialog, &dismiss_listener_field),
        Some(dismiss_listener)
    );
    assert_eq!(
        program.invoke_builtin_method(&is_showing, Some(&test_invocation(vec![dialog.clone()]))),
        Some(DexExecutionResult::Int(0))
    );
    assert!(program.write_stored_instance_field(&dialog, &controller_field, controller.clone()));
    assert!(program.write_stored_instance_field(
        &controller,
        &controller_title_field,
        DexValue::String(std::rc::Rc::from("Support Us"))
    ));
    assert!(program.write_stored_instance_field(
        &controller,
        &controller_button_field,
        DexValue::String(std::rc::Rc::from("Close"))
    ));
    assert!(program.write_stored_instance_field(
        &controller,
        &controller_view_field,
        dialog_content
    ));
    assert!(program.write_stored_instance_field(&context, &context_base_field, activity.clone()));
    assert!(program.write_stored_instance_field(
        &activity,
        &activity_history_field,
        DexValue::String(std::rc::Rc::from("09b8f2c5-727e-4eab-b439-d2110e320527"))
    ));
    assert_eq!(
        program.invoke_builtin_method(&show, Some(&test_invocation(vec![dialog.clone()]))),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&dialog, &showing_field),
        Some(DexValue::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&is_showing, Some(&test_invocation(vec![dialog.clone()]))),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.dialog_overlays(),
        vec![DexDialogOverlay {
            title: Some("Support Us".to_owned()),
            content_layouts: vec![2131492910],
            items: Vec::new(),
            buttons: vec![DexDialogButton {
                text: "Close".to_owned(),
                enabled: true,
            }],
        }]
    );
    assert_eq!(program.dialog_content_layouts(), vec![2131492910]);
    assert_eq!(
        program
            .click_dialog_button(0, 0)
            .expect("dialog button click should dismiss the dialog"),
        1
    );
    assert!(program.dialog_overlays().is_empty());
    assert_eq!(program.dialog_content_layouts(), vec![2131492910]);
    assert_eq!(
        program.invoke_builtin_method(&is_showing, Some(&test_invocation(vec![dialog]))),
        Some(DexExecutionResult::Int(0))
    );
}

#[test]
fn builtin_activity_set_theme_returns_void() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/app/Activity;".to_owned(),
        name: "setTheme".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let invocation = test_invocation(vec![
        DexValue::Object(DexObjectRef {
            id: 1,
            class_descriptor: std::rc::Rc::from(
                "Lcom/darkempire78/opencalculator/activities/MainActivity;".to_owned(),
            ),
        }),
        DexValue::Int(2131952000),
    ]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Void)
    );
}

#[test]
fn builtin_context_set_theme_returns_void() {
    // cite: frameworks-base Context.java#setTheme
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/content/Context;".to_owned(),
        name: "setTheme".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let invocation = test_invocation(vec![
        DexValue::Object(DexObjectRef {
            id: 1,
            class_descriptor: std::rc::Rc::from(
                "Lcom/darkempire78/opencalculator/activities/MainActivity;".to_owned(),
            ),
        }),
        DexValue::Int(2131952000),
    ]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Void)
    );
}

#[test]
fn builtin_activity_is_finishing_returns_false() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/app/Activity;".to_owned(),
        name: "isFinishing".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&method, None),
        Some(DexExecutionResult::Int(0))
    );
}

// cite: frameworks-base PhoneWindow.java#getDecorView, peekDecorView, installDecor, generateDecor
// cite: frameworks-base DecorView.java#DecorView
#[test]
fn frameworks_phone_window_decor_view_has_decor_view_runtime_type() {
    let program = DexProgram::new(Vec::new());
    let get_decor_view = DexMethodId {
        class_descriptor: "Landroid/view/Window;".to_owned(),
        name: "getDecorView".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/view/View;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let peek_decor_view = DexMethodId {
        class_descriptor: "Landroid/view/Window;".to_owned(),
        name: "peekDecorView".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/view/View;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get_callback = DexMethodId {
        class_descriptor: "Landroid/view/Window;".to_owned(),
        name: "getCallback".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/view/Window$Callback;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set_callback = DexMethodId {
        class_descriptor: "Landroid/view/Window;".to_owned(),
        name: "setCallback".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/view/Window$Callback;".to_owned()],
        },
    };
    let window = DexValue::Reference(std::rc::Rc::from("Landroid/view/Window;:dialog:test"));
    let callback =
        DexValue::Reference(std::rc::Rc::from("Landroid/view/Window$Callback;:delegate"));
    let decor = decor_view_reference();

    assert_eq!(
        program.invoke_builtin_method(&get_decor_view, None),
        Some(DexExecutionResult::Value(decor.clone()))
    );
    assert_eq!(
        program.invoke_builtin_method(&peek_decor_view, None),
        Some(DexExecutionResult::Value(decor.clone()))
    );
    assert!(program.value_matches_type(&decor, "Landroid/widget/FrameLayout;"));
    assert!(program.value_matches_type(&decor, "Landroid/view/ViewGroup;"));
    assert!(program.value_matches_type(&decor, "Landroid/view/View;"));
    assert_eq!(
        program.invoke_builtin_method(&get_callback, Some(&test_invocation(vec![window.clone()]))),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Landroid/view/Window$Callback;.bootstrap")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_callback,
            Some(&test_invocation(vec![window.clone(), callback.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&get_callback, Some(&test_invocation(vec![window]))),
        Some(DexExecutionResult::Value(callback))
    );
}

#[test]
fn builtin_window_attributes_are_window_scoped_and_replaceable() {
    let program = DexProgram::new(Vec::new());
    let window = DexValue::Reference(std::rc::Rc::from(
        "Landroid/view/Window;:activity:object:15",
    ));
    let get_attributes = DexMethodId {
        class_descriptor: "Landroid/view/Window;".to_owned(),
        name: "getAttributes".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/view/WindowManager$LayoutParams;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set_attributes = DexMethodId {
        class_descriptor: "Landroid/view/Window;".to_owned(),
        name: "setAttributes".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/view/WindowManager$LayoutParams;".to_owned()],
        },
    };
    let replacement = DexValue::Reference(std::rc::Rc::from(
        "Landroid/view/WindowManager$LayoutParams;:custom",
    ));

    assert_eq!(
        program.invoke_builtin_method(&get_attributes, Some(&test_invocation(vec![window.clone()]))),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Landroid/view/WindowManager$LayoutParams;:window:reference:Landroid/view/Window;:activity:object:15"
                .to_owned())
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_attributes,
            Some(&test_invocation(vec![window.clone(), replacement.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&get_attributes, Some(&test_invocation(vec![window]))),
        Some(DexExecutionResult::Value(replacement))
    );
}

#[test]
fn builtin_window_manager_layout_params_default_constructor_initializes_android_defaults() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Landroid/view/WindowManager$LayoutParams;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set_title = DexMethodId {
        class_descriptor: "Landroid/view/WindowManager$LayoutParams;".to_owned(),
        name: "setTitle".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/lang/CharSequence;".to_owned()],
        },
    };
    let get_title = DexMethodId {
        class_descriptor: "Landroid/view/WindowManager$LayoutParams;".to_owned(),
        name: "getTitle".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/CharSequence;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let layout_params = DexValue::Reference(std::rc::Rc::from(
        "Landroid/view/WindowManager$LayoutParams;:default",
    ));

    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![layout_params.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(
            &layout_params,
            &view_group_layout_params_int_field("width")
        ),
        Some(DexValue::Int(-1))
    );
    assert_eq!(
        program.read_stored_instance_field(
            &layout_params,
            &view_group_layout_params_int_field("height")
        ),
        Some(DexValue::Int(-1))
    );
    assert_eq!(
        program.read_stored_instance_field(
            &layout_params,
            &window_manager_layout_params_int_field("type")
        ),
        Some(DexValue::Int(2))
    );
    assert_eq!(
        program.read_stored_instance_field(
            &layout_params,
            &window_manager_layout_params_int_field("format")
        ),
        Some(DexValue::Int(-1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_title,
            Some(&test_invocation(vec![layout_params.clone()]))
        ),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from(String::new())
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_title,
            Some(&test_invocation(vec![
                layout_params.clone(),
                DexValue::String(std::rc::Rc::from("popup"))
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_title,
            Some(&test_invocation(vec![layout_params.clone()]))
        ),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("popup")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_title,
            Some(&test_invocation(vec![
                layout_params.clone(),
                DexValue::Null
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&get_title, Some(&test_invocation(vec![layout_params]))),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from(String::new())
        )))
    );
}

#[test]
fn builtin_window_callback_on_content_changed_returns_void() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/view/Window$Callback;".to_owned(),
        name: "onContentChanged".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&method, None),
        Some(DexExecutionResult::Void)
    );
}

#[test]
fn builtin_window_callback_on_create_panel_view_returns_null() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/view/Window$Callback;".to_owned(),
        name: "onCreatePanelView".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LI".to_owned(),
            return_type: "Landroid/view/View;".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::Reference(std::rc::Rc::from(
                    "Landroid/view/Window$Callback;.bootstrap"
                )),
                DexValue::Int(0),
            ]))
        ),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
}

#[test]
fn builtin_window_callback_on_create_panel_menu_returns_true() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/view/Window$Callback;".to_owned(),
        name: "onCreatePanelMenu".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZIL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["I".to_owned(), "Landroid/view/Menu;".to_owned()],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::Reference(std::rc::Rc::from(
                    "Landroid/view/Window$Callback;.bootstrap"
                )),
                DexValue::Int(0),
                DexValue::Reference(std::rc::Rc::from("Landroid/view/Menu;:panel")),
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
}

#[test]
fn builtin_window_callback_on_prepare_panel_returns_true() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/view/Window$Callback;".to_owned(),
        name: "onPreparePanel".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZILL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec![
                "I".to_owned(),
                "Landroid/view/View;".to_owned(),
                "Landroid/view/Menu;".to_owned(),
            ],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::Reference(std::rc::Rc::from(
                    "Landroid/view/Window$Callback;.bootstrap"
                )),
                DexValue::Int(0),
                DexValue::Null,
                DexValue::Reference(std::rc::Rc::from("Landroid/view/Menu;:panel")),
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
}

#[test]
fn builtin_window_add_flags_returns_void() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/view/Window;".to_owned(),
        name: "addFlags".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let invocation = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from("Landroid/view/Window;.bootstrap")),
        DexValue::Int(0x0000_0400),
    ]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Void)
    );
}

#[test]
fn builtin_window_bar_colors_round_trip() {
    let program = DexProgram::new(Vec::new());
    let window = DexValue::Reference(std::rc::Rc::from("Landroid/view/Window;.bootstrap"));
    let set_status_bar_color = DexMethodId {
        class_descriptor: "Landroid/view/Window;".to_owned(),
        name: "setStatusBarColor".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let get_status_bar_color = DexMethodId {
        class_descriptor: "Landroid/view/Window;".to_owned(),
        name: "getStatusBarColor".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set_navigation_bar_color = DexMethodId {
        class_descriptor: "Landroid/view/Window;".to_owned(),
        name: "setNavigationBarColor".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let get_navigation_bar_color = DexMethodId {
        class_descriptor: "Landroid/view/Window;".to_owned(),
        name: "getNavigationBarColor".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };

    assert_eq!(
        program.invoke_builtin_method(
            &get_status_bar_color,
            Some(&test_invocation(vec![window.clone()]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_status_bar_color,
            Some(&test_invocation(vec![window.clone(), DexValue::Int(-1)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_status_bar_color,
            Some(&test_invocation(vec![window.clone()]))
        ),
        Some(DexExecutionResult::Int(-1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_navigation_bar_color,
            Some(&test_invocation(vec![
                window.clone(),
                DexValue::Int(0xff00ff)
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_navigation_bar_color,
            Some(&test_invocation(vec![window]))
        ),
        Some(DexExecutionResult::Int(0xff00ff))
    );
}

#[test]
fn builtin_window_set_decor_fits_system_windows_stores_state() {
    let program = DexProgram::new(Vec::new());
    let window = DexValue::Reference(std::rc::Rc::from("Landroid/view/Window;.bootstrap"));
    let method = DexMethodId {
        class_descriptor: "Landroid/view/Window;".to_owned(),
        name: "setDecorFitsSystemWindows".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VZ".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Z".to_owned()],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![window.clone(), DexValue::Int(0)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&window, &window_decor_fits_system_windows_field()),
        Some(DexValue::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![window.clone(), DexValue::Int(1)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&window, &window_decor_fits_system_windows_field()),
        Some(DexValue::Int(1))
    );
}

#[test]
fn builtin_window_contrast_enforced_round_trips() {
    let program = DexProgram::new(Vec::new());
    let window = DexValue::Reference(std::rc::Rc::from("Landroid/view/Window;.bootstrap"));
    let set_status = DexMethodId {
        class_descriptor: "Landroid/view/Window;".to_owned(),
        name: "setStatusBarContrastEnforced".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VZ".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Z".to_owned()],
        },
    };
    let is_status = DexMethodId {
        class_descriptor: "Landroid/view/Window;".to_owned(),
        name: "isStatusBarContrastEnforced".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set_navigation = DexMethodId {
        class_descriptor: "Landroid/view/Window;".to_owned(),
        name: "setNavigationBarContrastEnforced".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VZ".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Z".to_owned()],
        },
    };
    let is_navigation = DexMethodId {
        class_descriptor: "Landroid/view/Window;".to_owned(),
        name: "isNavigationBarContrastEnforced".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&is_status, Some(&test_invocation(vec![window.clone()]))),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_status,
            Some(&test_invocation(vec![window.clone(), DexValue::Int(1)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&is_status, Some(&test_invocation(vec![window.clone()]))),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_navigation,
            Some(&test_invocation(vec![window.clone(), DexValue::Int(1)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&is_navigation, Some(&test_invocation(vec![window]))),
        Some(DexExecutionResult::Int(1))
    );
}

#[test]
fn builtin_window_get_insets_controller_returns_window_scoped_controller() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/view/Window;".to_owned(),
        name: "getInsetsController".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/view/WindowInsetsController;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let window = DexValue::Reference(std::rc::Rc::from(
        "Landroid/view/Window;:activity:object:15",
    ));

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![window]))),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Landroid/view/WindowInsetsController;:window:reference:Landroid/view/Window;:activity:object:15"
                .to_owned())
        )))
    );
}

#[test]
fn builtin_window_insets_controller_system_bars_appearance_uses_mask() {
    let program = DexProgram::new(Vec::new());
    let controller = DexValue::Reference(std::rc::Rc::from(
        "Landroid/view/WindowInsetsController;:window:test",
    ));
    let set_appearance = DexMethodId {
        class_descriptor: "Landroid/view/WindowInsetsController;".to_owned(),
        name: "setSystemBarsAppearance".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VII".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned(), "I".to_owned()],
        },
    };
    let get_appearance = DexMethodId {
        class_descriptor: "Landroid/view/WindowInsetsController;".to_owned(),
        name: "getSystemBarsAppearance".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };

    assert_eq!(
        program.invoke_builtin_method(
            &set_appearance,
            Some(&test_invocation(vec![
                controller.clone(),
                DexValue::Int(0b1010),
                DexValue::Int(0b1111),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_appearance,
            Some(&test_invocation(vec![
                controller.clone(),
                DexValue::Int(0b0101),
                DexValue::Int(0b0011),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&get_appearance, Some(&test_invocation(vec![controller]))),
        Some(DexExecutionResult::Int(0b1001))
    );
}

#[test]
fn builtin_window_insets_controller_show_accepts_type_mask() {
    let program = DexProgram::new(Vec::new());
    let show = DexMethodId {
        class_descriptor: "Landroid/view/WindowInsetsController;".to_owned(),
        name: "show".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let controller = DexValue::Reference(std::rc::Rc::from(
        "Landroid/view/WindowInsetsController;:window:test",
    ));

    assert_eq!(
        program.invoke_builtin_method(
            &show,
            Some(&test_invocation(vec![controller, DexValue::Int(8)]))
        ),
        Some(DexExecutionResult::Void)
    );
}

#[test]
// cite: frameworks-base Window.java#findViewById
// cite: frameworks-base View.java#findViewById, findViewTraversal
fn builtin_window_find_view_by_id_returns_null_when_id_is_absent() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/view/Window;".to_owned(),
        name: "findViewById".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LI".to_owned(),
            return_type: "Landroid/view/View;".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let invocation = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from("Landroid/view/Window;.bootstrap")),
        DexValue::Int(2131296365),
    ]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    assert_eq!(program.view_snapshot().view_lookup_ids, vec![2131296365]);
}

#[test]
fn builtin_window_set_content_view_returns_void() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/view/Window;".to_owned(),
        name: "setContentView".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/view/View;".to_owned()],
        },
    };
    let invocation = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from("Landroid/view/Window;.bootstrap")),
        DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:layout/2131492887")),
    ]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Void)
    );
}

#[test]
fn builtin_activity_set_content_view_attaches_view_to_content_root() {
    let program = DexProgram::new(Vec::new());
    let set_content_view = DexMethodId {
        class_descriptor: "Landroid/app/Activity;".to_owned(),
        name: "setContentView".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VLL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "Landroid/view/View;".to_owned(),
                "Landroid/view/ViewGroup$LayoutParams;".to_owned(),
            ],
        },
    };
    let get_parent = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "getParent".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/view/ViewParent;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get_layout_params = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "getLayoutParams".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/view/ViewGroup$LayoutParams;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let activity = DexValue::Reference(std::rc::Rc::from("Landroid/app/Activity;:test"));
    let view = DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:compose"));
    let layout_params = DexValue::Reference(std::rc::Rc::from(
        "Landroid/view/ViewGroup$LayoutParams;:compose",
    ));

    assert_eq!(
        program.invoke_builtin_method(
            &set_content_view,
            Some(&test_invocation(vec![
                activity.clone(),
                view.clone(),
                layout_params.clone()
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&get_parent, Some(&test_invocation(vec![view.clone()]))),
        Some(DexExecutionResult::Value(view_reference(
            ANDROID_CONTENT_VIEW_ID
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&get_layout_params, Some(&test_invocation(vec![view]))),
        Some(DexExecutionResult::Value(layout_params))
    );

    assert_eq!(
        program.view_snapshot().runtime_view_roots,
        vec![DexRuntimeViewRoot {
            view_key: "reference:Landroid/view/View;:compose".to_owned(),
            class_descriptor: "Landroid/view/View;".to_owned(),
        }]
    );

    let layout_root = layout_reference(42);
    let second_layout_params = DexValue::Reference(std::rc::Rc::from(
        "Landroid/view/ViewGroup$LayoutParams;:layout",
    ));

    assert_eq!(
        program.invoke_builtin_method(
            &set_content_view,
            Some(&test_invocation(vec![
                activity,
                layout_root,
                second_layout_params
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert!(program.view_snapshot().runtime_view_roots.is_empty());
}

#[test]
fn builtin_window_manager_get_default_display_returns_bootstrap_display() {
    let program = DexProgram::new(Vec::new());
    let get_default_display = DexMethodId {
        class_descriptor: "Landroid/view/WindowManager;".to_owned(),
        name: "getDefaultDisplay".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/view/Display;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get_current_window_metrics = DexMethodId {
        class_descriptor: "Landroid/view/WindowManager;".to_owned(),
        name: "getCurrentWindowMetrics".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/view/WindowMetrics;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get_bounds = DexMethodId {
        class_descriptor: "Landroid/view/WindowMetrics;".to_owned(),
        name: "getBounds".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/graphics/Rect;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get_size = DexMethodId {
        class_descriptor: "Landroid/view/Display;".to_owned(),
        name: "getSize".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/graphics/Point;".to_owned()],
        },
    };
    let get_real_size = DexMethodId {
        class_descriptor: "Landroid/view/Display;".to_owned(),
        name: "getRealSize".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/graphics/Point;".to_owned()],
        },
    };
    let get_width = DexMethodId {
        class_descriptor: "Landroid/view/Display;".to_owned(),
        name: "getWidth".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get_height = DexMethodId {
        class_descriptor: "Landroid/view/Display;".to_owned(),
        name: "getHeight".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get_rotation = DexMethodId {
        class_descriptor: "Landroid/view/Display;".to_owned(),
        name: "getRotation".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get_refresh_rate = DexMethodId {
        class_descriptor: "Landroid/view/Display;".to_owned(),
        name: "getRefreshRate".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "F".to_owned(),
            return_type: "F".to_owned(),
            parameters: Vec::new(),
        },
    };
    let point_constructor = DexMethodId {
        class_descriptor: "Landroid/graphics/Point;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let point = DexValue::Reference(std::rc::Rc::from("Landroid/graphics/Point;:display"));
    let display = DexValue::Reference(std::rc::Rc::from("Landroid/view/Display;.bootstrap"));
    let window_manager =
        DexValue::Reference(std::rc::Rc::from("Landroid/view/WindowManager;:main"));

    assert_eq!(
        program.invoke_builtin_method(&get_default_display, None),
        Some(DexExecutionResult::Value(display.clone()))
    );
    let metrics = match program.invoke_builtin_method(
        &get_current_window_metrics,
        Some(&test_invocation(vec![window_manager])),
    ) {
        Some(DexExecutionResult::Value(value)) => value,
        result => panic!("unexpected WindowManager.getCurrentWindowMetrics result: {result:?}"),
    };
    let bounds =
        match program.invoke_builtin_method(&get_bounds, Some(&test_invocation(vec![metrics]))) {
            Some(DexExecutionResult::Value(value)) => value,
            result => panic!("unexpected WindowMetrics.getBounds result: {result:?}"),
        };
    assert_eq!(
        program.read_stored_instance_field(&bounds, &rect_int_field("left")),
        Some(DexValue::Int(0))
    );
    assert_eq!(
        program.read_stored_instance_field(&bounds, &rect_int_field("top")),
        Some(DexValue::Int(0))
    );
    assert_eq!(
        program.read_stored_instance_field(&bounds, &rect_int_field("right")),
        Some(DexValue::Int(BOOTSTRAP_DISPLAY_WIDTH))
    );
    assert_eq!(
        program.read_stored_instance_field(&bounds, &rect_int_field("bottom")),
        Some(DexValue::Int(BOOTSTRAP_DISPLAY_HEIGHT))
    );
    assert_eq!(
        program.invoke_builtin_method(&point_constructor, None),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_size,
            Some(&test_invocation(vec![display.clone(), point.clone()])),
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&point, &point_int_field("x")),
        Some(DexValue::Int(BOOTSTRAP_DISPLAY_WIDTH))
    );
    assert_eq!(
        program.read_stored_instance_field(&point, &point_int_field("y")),
        Some(DexValue::Int(BOOTSTRAP_DISPLAY_HEIGHT))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_real_size,
            Some(&test_invocation(vec![display.clone(), point.clone()])),
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&get_width, Some(&test_invocation(vec![display.clone()]))),
        Some(DexExecutionResult::Int(BOOTSTRAP_DISPLAY_WIDTH))
    );
    assert_eq!(
        program.invoke_builtin_method(&get_height, Some(&test_invocation(vec![display.clone()]))),
        Some(DexExecutionResult::Int(BOOTSTRAP_DISPLAY_HEIGHT))
    );
    assert_eq!(
        program.invoke_builtin_method(&get_rotation, Some(&test_invocation(vec![display]))),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_refresh_rate,
            Some(&test_invocation(vec![DexValue::Reference(
                std::rc::Rc::from("Landroid/view/Display;.bootstrap")
            )]))
        ),
        Some(DexExecutionResult::Value(DexValue::Float(60.0)))
    );
}

// cite: frameworks-base Activity.java#setRequestedOrientation, getRequestedOrientation
// cite: frameworks-base ActivityInfo.java#SCREEN_ORIENTATION_UNSPECIFIED
#[test]
fn activity_requested_orientation_defaults_to_unspecified_and_round_trips() {
    let program = DexProgram::new(Vec::new());
    let activity = program.allocate_object("Lcom/example/MainActivity;".to_owned());
    let getter = DexMethodId {
        class_descriptor: "Landroid/app/Activity;".to_owned(),
        name: "getRequestedOrientation".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let setter = DexMethodId {
        class_descriptor: "Landroid/app/Activity;".to_owned(),
        name: "setRequestedOrientation".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&getter, Some(&test_invocation(vec![activity.clone()]))),
        Some(DexExecutionResult::Int(-1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &setter,
            Some(&test_invocation(vec![activity.clone(), DexValue::Int(1)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&getter, Some(&test_invocation(vec![activity]))),
        Some(DexExecutionResult::Int(1))
    );
}

#[test]
fn builtin_boolean_hash_code_matches_java_constants() {
    // cite: libcore Boolean.java#hashCode
    let program = DexProgram::new(Vec::new());
    let static_hash_code = DexMethodId {
        class_descriptor: "Ljava/lang/Boolean;".to_owned(),
        name: "hashCode".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "IZ".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["Z".to_owned()],
        },
    };
    let instance_hash_code = DexMethodId {
        class_descriptor: "Ljava/lang/Boolean;".to_owned(),
        name: "hashCode".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };

    assert_eq!(
        program.invoke_builtin_method(
            &static_hash_code,
            Some(&test_invocation(vec![DexValue::Int(1)]))
        ),
        Some(DexExecutionResult::Int(1231))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &static_hash_code,
            Some(&test_invocation(vec![DexValue::Int(0)]))
        ),
        Some(DexExecutionResult::Int(1237))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &instance_hash_code,
            Some(&test_invocation(vec![DexValue::Reference(
                std::rc::Rc::from("Ljava/lang/Boolean;:true")
            )]))
        ),
        Some(DexExecutionResult::Int(1231))
    );
}

#[test]
fn builtin_accessibility_manager_recommended_timeout_defaults_to_original() {
    // cite: frameworks-base AccessibilityManager.java#getRecommendedTimeoutMillis
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/view/accessibility/AccessibilityManager;".to_owned(),
        name: "getRecommendedTimeoutMillis".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "III".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["I".to_owned(), "I".to_owned()],
        },
    };
    let manager = DexValue::Reference(std::rc::Rc::from(
        "Landroid/view/accessibility/AccessibilityManager;.bootstrap",
    ));

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                manager,
                DexValue::Int(2000),
                DexValue::Int(7)
            ]))
        ),
        Some(DexExecutionResult::Int(2000))
    );
}

#[test]
fn builtin_dialog_single_arg_constructor_delegates_to_default_theme() {
    // cite: frameworks-base Dialog.java#Dialog
    let program = DexProgram::new(Vec::new());
    let dialog = DexValue::Object(DexObjectRef {
        id: 804,
        class_descriptor: std::rc::Rc::from("Landroid/app/Dialog;"),
    });
    let context = DexValue::Object(DexObjectRef {
        id: 805,
        class_descriptor: std::rc::Rc::from("Landroid/app/Activity;"),
    });
    let init = DexMethodId {
        class_descriptor: "Landroid/app/Dialog;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/content/Context;".to_owned()],
        },
    };
    let get_context = DexMethodId {
        class_descriptor: "Landroid/app/Dialog;".to_owned(),
        name: "getContext".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/content/Context;".to_owned(),
            parameters: Vec::new(),
        },
    };

    assert_eq!(
        program.invoke_builtin_method(
            &init,
            Some(&test_invocation(vec![dialog.clone(), context.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&get_context, Some(&test_invocation(vec![dialog]))),
        Some(DexExecutionResult::Value(context))
    );
}

#[test]
fn builtin_dialog_set_content_view_by_resource_inflates_and_links_decor() {
    // cite: frameworks-base Dialog.java#setContentView
    // cite: frameworks-base PhoneWindow.java#setContentView
    let program = DexProgram::new(Vec::new());
    let dialog = DexValue::Object(DexObjectRef {
        id: 806,
        class_descriptor: std::rc::Rc::from("Landroid/app/Dialog;"),
    });
    let set_content_view = DexMethodId {
        class_descriptor: "Landroid/app/Dialog;".to_owned(),
        name: "setContentView".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let get_parent = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "getParent".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/view/ViewParent;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let content_view =
        DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:layout/2131492910"));

    assert_eq!(
        program.invoke_builtin_method(
            &set_content_view,
            Some(&test_invocation(vec![dialog, DexValue::Int(2131492910)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&get_parent, Some(&test_invocation(vec![content_view]))),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Lcom/android/internal/policy/DecorView;.decor")
        )))
    );
}

#[test]
fn builtin_dialog_find_view_by_id_returns_registered_inflated_child() {
    // cite: frameworks-base Dialog.java#findViewById
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/app/Dialog;".to_owned(),
        name: "findViewById".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LI".to_owned(),
            return_type: "Landroid/view/View;".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let dialog = DexValue::Object(DexObjectRef {
        id: 807,
        class_descriptor: std::rc::Rc::from("Landroid/app/Dialog;"),
    });
    let child = DexValue::Reference(std::rc::Rc::from(
        "Landroid/widget/TextView;:layout/1/id/16908310",
    ));

    program
        .runtime
        .borrow_mut()
        .inflated_view_children
        .insert((map_identity_key(&dialog), 16908310), child.clone());

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![dialog, DexValue::Int(16908310)]))
        ),
        Some(DexExecutionResult::Value(child))
    );
}

#[test]
fn builtin_dialog_find_view_by_id_resolves_through_unshown_content_view() {
    // cite: frameworks-base Dialog.java#setContentView, findViewById
    let program = DexProgram::new(Vec::new());
    let dialog = DexValue::Object(DexObjectRef {
        id: 808,
        class_descriptor: std::rc::Rc::from("Landroid/app/Dialog;"),
    });
    let set_content_view = DexMethodId {
        class_descriptor: "Landroid/app/Dialog;".to_owned(),
        name: "setContentView".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let find_view_by_id = DexMethodId {
        class_descriptor: "Landroid/app/Dialog;".to_owned(),
        name: "findViewById".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LI".to_owned(),
            return_type: "Landroid/view/View;".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let root = layout_reference(2130968578);
    let child = layout_child_reference(2130968578, 2131034134, "Landroid/widget/Button;");

    assert_eq!(
        program.invoke_builtin_method(
            &set_content_view,
            Some(&test_invocation(vec![
                dialog.clone(),
                DexValue::Int(2130968578)
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    program
        .runtime
        .borrow_mut()
        .inflated_view_children
        .insert((map_identity_key(&root), 2131034134), child.clone());

    assert_eq!(
        program.invoke_builtin_method(
            &find_view_by_id,
            Some(&test_invocation(vec![dialog, DexValue::Int(2131034134)]))
        ),
        Some(DexExecutionResult::Value(child))
    );
}

#[test]
fn builtin_dialog_lifecycle_hooks_and_back_dispatcher_no_op_correctly() {
    // cite: frameworks-base Dialog.java#onCreate, onStart, onStop, getOnBackInvokedDispatcher
    let program = DexProgram::new(Vec::new());
    let dialog = DexValue::Object(DexObjectRef {
        id: 801,
        class_descriptor: std::rc::Rc::from("Landroid/app/Dialog;"),
    });
    let void_method = |name: &str, descriptor: &str| DexMethodId {
        class_descriptor: "Landroid/app/Dialog;".to_owned(),
        name: name.to_owned(),
        proto: crate::dex::DexProto {
            shorty: if descriptor == "()V" { "V" } else { "VL" }.to_owned(),
            return_type: "V".to_owned(),
            parameters: if descriptor == "()V" {
                Vec::new()
            } else {
                vec!["Landroid/os/Bundle;".to_owned()]
            },
        },
    };

    assert_eq!(
        program.invoke_builtin_method(
            &void_method("onCreate", "(Landroid/os/Bundle;)V"),
            Some(&test_invocation(vec![dialog.clone(), DexValue::Null]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &void_method("onStart", "()V"),
            Some(&test_invocation(vec![dialog.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &void_method("onStop", "()V"),
            Some(&test_invocation(vec![dialog.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );

    let get_dispatcher = DexMethodId {
        class_descriptor: "Landroid/app/Dialog;".to_owned(),
        name: "getOnBackInvokedDispatcher".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/window/OnBackInvokedDispatcher;".to_owned(),
            parameters: Vec::new(),
        },
    };
    assert_eq!(
        program.invoke_builtin_method(&get_dispatcher, Some(&test_invocation(vec![dialog]))),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Landroid/window/OnBackInvokedDispatcher;.bootstrap")
        )))
    );
}

#[test]
fn builtin_dialog_set_content_view_links_content_view_to_decor() {
    // cite: frameworks-base Window.java#setContentView
    let program = DexProgram::new(Vec::new());
    let dialog = DexValue::Object(DexObjectRef {
        id: 802,
        class_descriptor: std::rc::Rc::from("Landroid/app/Dialog;"),
    });
    let content_view = DexValue::Object(DexObjectRef {
        id: 803,
        class_descriptor: std::rc::Rc::from("Landroid/view/View;"),
    });
    let set_content_view = DexMethodId {
        class_descriptor: "Landroid/app/Dialog;".to_owned(),
        name: "setContentView".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VLL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/view/View;".to_owned()],
        },
    };
    let get_parent = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "getParent".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/view/ViewParent;".to_owned(),
            parameters: Vec::new(),
        },
    };

    assert_eq!(
        program.invoke_builtin_method(
            &set_content_view,
            Some(&test_invocation(vec![dialog, content_view.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&get_parent, Some(&test_invocation(vec![content_view]))),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Lcom/android/internal/policy/DecorView;.decor")
        )))
    );
}

// cite: frameworks-base WindowManager.java#isCrossWindowBlurEnabled
#[test]
fn builtin_window_manager_cross_window_blur_is_disabled() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/view/WindowManager;".to_owned(),
        name: "isCrossWindowBlurEnabled".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&method, None),
        Some(DexExecutionResult::Int(0))
    );
}

// cite: frameworks-base Activity.java#getIntent, setIntent
#[test]
fn builtin_activity_set_intent_replaces_get_intent() {
    let program = DexProgram::new(Vec::new());
    let activity_method = |name: &str, shorty: &str, ret: &str, params: Vec<String>| DexMethodId {
        class_descriptor: "Landroid/app/Activity;".to_owned(),
        name: name.to_owned(),
        proto: crate::dex::DexProto {
            shorty: shorty.to_owned(),
            return_type: ret.to_owned(),
            parameters: params,
        },
    };
    let get_intent = activity_method("getIntent", "L", "Landroid/content/Intent;", Vec::new());
    let set_intent = activity_method(
        "setIntent",
        "VL",
        "V",
        vec!["Landroid/content/Intent;".to_owned()],
    );
    let activity = program.allocate_object("Lcom/example/MainActivity;".to_owned());
    let intent = program.allocate_object("Landroid/content/Intent;".to_owned());

    assert_eq!(
        program.invoke_builtin_method(&get_intent, Some(&test_invocation(vec![activity.clone()]))),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Landroid/content/Intent;.bootstrap")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_intent,
            Some(&test_invocation(vec![activity.clone(), intent.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&get_intent, Some(&test_invocation(vec![activity]))),
        Some(DexExecutionResult::Value(intent))
    );
}
