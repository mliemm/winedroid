use super::super::*;
use super::helpers::*;

#[test]
fn adapter_item_click_registry_round_trips() {
    let program = DexProgram::new(Vec::new());
    let list_view = DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:id/100"));

    let first = program.register_adapter_item_click(&list_view, 100, 0);
    let second = program.register_adapter_item_click(&list_view, 100, 1);

    assert_ne!(first, second, "each row gets a distinct click id");
    assert!((ADAPTER_ITEM_CLICK_ID_BASE..0x7000_0000).contains(&first));
    assert_eq!(
        program.adapter_item_click(first),
        Some((list_view.clone(), 0))
    );
    assert_eq!(
        program.adapter_item_click(second),
        Some((list_view.clone(), 1))
    );

    program.clear_adapter_item_clicks(100);
    assert_eq!(program.adapter_item_click(first), None);
}

#[test]
fn builtin_view_clear_focus_clears_focused_state() {
    let program = DexProgram::new(Vec::new());
    let view = DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:id/1"));
    let void_proto = || crate::dex::DexProto {
        shorty: "V".to_owned(),
        return_type: "V".to_owned(),
        parameters: Vec::new(),
    };
    let bool_proto = || crate::dex::DexProto {
        shorty: "Z".to_owned(),
        return_type: "Z".to_owned(),
        parameters: Vec::new(),
    };
    let request_focus = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "requestFocus".to_owned(),
        proto: bool_proto(),
    };
    let clear_focus = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "clearFocus".to_owned(),
        proto: void_proto(),
    };
    let is_focused = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "isFocused".to_owned(),
        proto: bool_proto(),
    };

    assert_eq!(
        program.invoke_builtin_method(&request_focus, Some(&test_invocation(vec![view.clone()]))),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&is_focused, Some(&test_invocation(vec![view.clone()]))),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&clear_focus, Some(&test_invocation(vec![view.clone()]))),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&is_focused, Some(&test_invocation(vec![view]))),
        Some(DexExecutionResult::Int(0))
    );
}

#[test]
fn builtin_view_outline_provider_constructor_is_noop() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/view/ViewOutlineProvider;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set_outline_provider = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "setOutlineProvider".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/view/ViewOutlineProvider;".to_owned()],
        },
    };
    let get_outline_provider = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "getOutlineProvider".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/view/ViewOutlineProvider;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let view = DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:outline"));
    let outline_provider = DexValue::Reference(std::rc::Rc::from("outline-provider"));

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![outline_provider.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_outline_provider,
            Some(&test_invocation(vec![view.clone()]))
        ),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_outline_provider,
            Some(&test_invocation(vec![
                view.clone(),
                outline_provider.clone()
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_outline_provider,
            Some(&test_invocation(vec![view.clone()]))
        ),
        Some(DexExecutionResult::Value(outline_provider))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_outline_provider,
            Some(&test_invocation(vec![view.clone(), DexValue::Null]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&get_outline_provider, Some(&test_invocation(vec![view]))),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
}

#[test]
fn builtin_view_tags_store_and_read_values() {
    let program = DexProgram::new(Vec::new());
    let set_tag = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "setTag".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VIL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned(), "Ljava/lang/Object;".to_owned()],
        },
    };
    let get_tag = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "getTag".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LI".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let view = DexValue::Reference(std::rc::Rc::from(
        "Lcom/android/internal/policy/DecorView;.decor",
    ));
    let value = DexValue::Reference(std::rc::Rc::from("owner"));
    let set_invocation = test_invocation(vec![view.clone(), DexValue::Int(42), value.clone()]);
    let get_invocation = test_invocation(vec![view.clone(), DexValue::Int(42)]);
    let missing_invocation = test_invocation(vec![view, DexValue::Int(43)]);

    assert_eq!(
        program.invoke_builtin_method(&get_tag, Some(&missing_invocation)),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    assert_eq!(
        program.invoke_builtin_method(&set_tag, Some(&set_invocation)),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&get_tag, Some(&get_invocation)),
        Some(DexExecutionResult::Value(value))
    );
}

#[test]
fn builtin_view_default_tag_stores_and_reads_value() {
    let program = DexProgram::new(Vec::new());
    let set_tag = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "setTag".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let get_tag = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "getTag".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let view = DexValue::Reference(std::rc::Rc::from(
        "Lcom/android/internal/policy/DecorView;.decor",
    ));
    let value = DexValue::Reference(std::rc::Rc::from("owner"));

    assert_eq!(
        program.invoke_builtin_method(&get_tag, Some(&test_invocation(vec![view.clone()]))),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_tag,
            Some(&test_invocation(vec![view.clone(), value.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&get_tag, Some(&test_invocation(vec![view]))),
        Some(DexExecutionResult::Value(value))
    );
}

#[test]
fn builtin_view_get_display_returns_bootstrap_display() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "getDisplay".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/view/Display;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let view = DexValue::Object(DexObjectRef {
        id: 44,
        class_descriptor: std::rc::Rc::from("Landroid/view/View;"),
    });

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![view]))),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Landroid/view/Display;.bootstrap")
        )))
    );
}

#[test]
fn builtin_view_is_in_edit_mode_returns_false() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "isInEditMode".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let view = DexValue::Object(DexObjectRef {
        id: 44,
        class_descriptor: std::rc::Rc::from("Landroid/view/View;"),
    });

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![view]))),
        Some(DexExecutionResult::Int(0))
    );
}

#[test]
fn builtin_view_unique_drawing_id_uses_object_identity() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "getUniqueDrawingId".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "J".to_owned(),
            return_type: "J".to_owned(),
            parameters: Vec::new(),
        },
    };
    let view = DexValue::Object(DexObjectRef {
        id: 42,
        class_descriptor: std::rc::Rc::from("Landroid/view/View;"),
    });

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![view]))),
        Some(DexExecutionResult::Value(DexValue::Long(42)))
    );
}

#[test]
fn builtin_view_parent_on_descendant_invalidated_accepts_notification() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/view/ViewParent;".to_owned(),
        name: "onDescendantInvalidated".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VLL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "Landroid/view/View;".to_owned(),
                "Landroid/view/View;".to_owned(),
            ],
        },
    };
    let parent = DexValue::Reference(std::rc::Rc::from(
        "Lcom/android/internal/policy/DecorView;.decor",
    ));
    let child = DexValue::Object(DexObjectRef {
        id: 44,
        class_descriptor: std::rc::Rc::from("Landroid/view/View;"),
    });

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![parent, child.clone(), child]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert!(program.runtime.borrow().has_traversal_request());
}

#[test]
fn androidx_view_tree_owner_tag_names_map_to_bootstrap_owners() {
    assert_eq!(
        androidx_view_tree_owner_tag_value("id", "view_tree_lifecycle_owner"),
        Some(DexValue::Reference(std::rc::Rc::from(
            "Landroidx/lifecycle/LifecycleOwner;.bootstrap"
        )))
    );
    assert_eq!(
        androidx_view_tree_owner_tag_value("id", "view_tree_navigation_event_dispatcher_owner"),
        Some(DexValue::Reference(std::rc::Rc::from(
            "Landroidx/navigationevent/NavigationEventDispatcherOwner;.bootstrap"
        )))
    );
    assert_eq!(
        androidx_view_tree_owner_tag_value("id", "view_tree_on_back_pressed_dispatcher_owner"),
        Some(DexValue::Reference(std::rc::Rc::from(
            "Landroidx/activity/OnBackPressedDispatcherOwner;.bootstrap"
        )))
    );
    assert_eq!(
        androidx_view_tree_owner_tag_value("id", "view_tree_saved_state_registry_owner"),
        Some(DexValue::Reference(std::rc::Rc::from(
            "Landroidx/savedstate/SavedStateRegistryOwner;.bootstrap"
        )))
    );
    assert_eq!(
        androidx_view_tree_owner_tag_value("id", "view_tree_view_model_store_owner"),
        Some(DexValue::Reference(std::rc::Rc::from(
            "Landroidx/lifecycle/ViewModelStoreOwner;.bootstrap"
        )))
    );
    assert_eq!(
        androidx_view_tree_owner_tag_value("string", "view_tree_lifecycle_owner"),
        None
    );
    assert_eq!(androidx_view_tree_owner_tag_value("id", "other"), None);
}

#[test]
fn builtin_view_get_parent_returns_null_for_bootstrap_view() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "getParent".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/view/ViewParent;".to_owned(),
            parameters: Vec::new(),
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&method, None),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
}

// cite: frameworks-base PhoneWindow.java#generateLayout, ID_ANDROID_CONTENT
// cite: frameworks-base screen.xml#@android:id/content
// cite: frameworks-base screen_action_bar.xml#@android:id/content
// cite: frameworks-base screen_simple.xml#@android:id/content
// cite: frameworks-base screen_toolbar.xml#@android:id/content
#[test]
fn frameworks_phone_window_content_container_has_frame_layout_runtime_type() {
    let program = DexProgram::new(Vec::new());
    let content = view_reference(ANDROID_CONTENT_VIEW_ID);

    assert_eq!(
        content,
        DexValue::Reference(std::rc::Rc::from(format!(
            "Landroid/widget/FrameLayout;:id/{ANDROID_CONTENT_VIEW_ID}"
        )))
    );
    assert!(program.value_matches_type(&content, "Landroid/widget/FrameLayout;"));
    assert!(program.value_matches_type(&content, "Landroid/view/ViewGroup;"));
    assert!(program.value_matches_type(&content, "Landroid/view/View;"));
}

#[test]
fn builtin_view_get_root_window_insets_returns_root_scoped_value() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "getRootWindowInsets".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/view/WindowInsets;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let child = DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:child"));
    let content = view_reference(ANDROID_CONTENT_VIEW_ID);

    program
        .runtime
        .borrow_mut()
        .view_parents
        .insert(map_identity_key(&child), content.clone());

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![child]))),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from(format!(
                "Landroid/view/WindowInsets;:root:{}",
                map_identity_key(&content)
            ))
        )))
    );
}

#[test]
fn builtin_view_get_window_insets_controller_returns_view_scoped_controller() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "getWindowInsetsController".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/view/WindowInsetsController;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let view = DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:decor"));

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![view]))),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from(
                "Landroid/view/WindowInsetsController;:view:reference:Landroid/view/View;:decor"
                    .to_owned()
            )
        )))
    );
}

#[test]
fn builtin_view_tree_observer_supports_layout_listener_bootstrap() {
    let program = DexProgram::new(Vec::new());
    let get_view_tree_observer = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "getViewTreeObserver".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/view/ViewTreeObserver;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let is_alive = DexMethodId {
        class_descriptor: "Landroid/view/ViewTreeObserver;".to_owned(),
        name: "isAlive".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let add_listener = DexMethodId {
        class_descriptor: "Landroid/view/ViewTreeObserver;".to_owned(),
        name: "addOnGlobalLayoutListener".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/view/ViewTreeObserver$OnGlobalLayoutListener;".to_owned()],
        },
    };
    let remove_listener = DexMethodId {
        class_descriptor: "Landroid/view/ViewTreeObserver;".to_owned(),
        name: "removeOnGlobalLayoutListener".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/view/ViewTreeObserver$OnGlobalLayoutListener;".to_owned()],
        },
    };
    let add_draw_listener = DexMethodId {
        class_descriptor: "Landroid/view/ViewTreeObserver;".to_owned(),
        name: "addOnDrawListener".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/view/ViewTreeObserver$OnDrawListener;".to_owned()],
        },
    };
    let remove_draw_listener = DexMethodId {
        class_descriptor: "Landroid/view/ViewTreeObserver;".to_owned(),
        name: "removeOnDrawListener".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/view/ViewTreeObserver$OnDrawListener;".to_owned()],
        },
    };
    let add_scroll_listener = DexMethodId {
        class_descriptor: "Landroid/view/ViewTreeObserver;".to_owned(),
        name: "addOnScrollChangedListener".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/view/ViewTreeObserver$OnScrollChangedListener;".to_owned()],
        },
    };
    let remove_scroll_listener = DexMethodId {
        class_descriptor: "Landroid/view/ViewTreeObserver;".to_owned(),
        name: "removeOnScrollChangedListener".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/view/ViewTreeObserver$OnScrollChangedListener;".to_owned()],
        },
    };
    let add_window_focus_listener = DexMethodId {
        class_descriptor: "Landroid/view/ViewTreeObserver;".to_owned(),
        name: "addOnWindowFocusChangeListener".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "Landroid/view/ViewTreeObserver$OnWindowFocusChangeListener;".to_owned(),
            ],
        },
    };
    let remove_window_focus_listener = DexMethodId {
        class_descriptor: "Landroid/view/ViewTreeObserver;".to_owned(),
        name: "removeOnWindowFocusChangeListener".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "Landroid/view/ViewTreeObserver$OnWindowFocusChangeListener;".to_owned(),
            ],
        },
    };
    let add_touch_mode_listener = DexMethodId {
        class_descriptor: "Landroid/view/ViewTreeObserver;".to_owned(),
        name: "addOnTouchModeChangeListener".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/view/ViewTreeObserver$OnTouchModeChangeListener;".to_owned()],
        },
    };
    let remove_touch_mode_listener = DexMethodId {
        class_descriptor: "Landroid/view/ViewTreeObserver;".to_owned(),
        name: "removeOnTouchModeChangeListener".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/view/ViewTreeObserver$OnTouchModeChangeListener;".to_owned()],
        },
    };
    let view = DexValue::Reference(std::rc::Rc::from(
        "Lcom/android/internal/policy/DecorView;.decor",
    ));
    let observer = DexValue::Reference(std::rc::Rc::from(
        "Landroid/view/ViewTreeObserver;:reference:Lcom/android/internal/policy/DecorView;.decor",
    ));
    let global_layout_listener = DexValue::Reference(std::rc::Rc::from("global-layout-listener"));
    let draw_listener = DexValue::Reference(std::rc::Rc::from("draw-listener"));
    let scroll_listener = DexValue::Reference(std::rc::Rc::from("scroll-listener"));
    let window_focus_listener = DexValue::Reference(std::rc::Rc::from("window-focus-listener"));
    let touch_mode_listener = DexValue::Reference(std::rc::Rc::from("touch-mode-listener"));

    assert_eq!(
        program.invoke_builtin_method(&get_view_tree_observer, Some(&test_invocation(vec![view]))),
        Some(DexExecutionResult::Value(observer.clone()))
    );
    assert_eq!(
        program.invoke_builtin_method(&is_alive, Some(&test_invocation(vec![observer.clone()]))),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &add_listener,
            Some(&test_invocation(vec![
                observer.clone(),
                global_layout_listener.clone()
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &add_draw_listener,
            Some(&test_invocation(vec![
                observer.clone(),
                draw_listener.clone()
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &add_scroll_listener,
            Some(&test_invocation(vec![
                observer.clone(),
                scroll_listener.clone()
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &add_window_focus_listener,
            Some(&test_invocation(vec![
                observer.clone(),
                window_focus_listener.clone()
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &add_touch_mode_listener,
            Some(&test_invocation(vec![
                observer.clone(),
                touch_mode_listener.clone()
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .view_tree_global_layout_listeners
            .get(&map_identity_key(&observer))
            .cloned(),
        Some(vec![global_layout_listener.clone()])
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .view_tree_draw_listeners
            .get(&map_identity_key(&observer))
            .cloned(),
        Some(vec![draw_listener.clone()])
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .view_tree_scroll_changed_listeners
            .get(&map_identity_key(&observer))
            .cloned(),
        Some(vec![scroll_listener.clone()])
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .view_tree_window_focus_change_listeners
            .get(&map_identity_key(&observer))
            .cloned(),
        Some(vec![window_focus_listener.clone()])
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .view_tree_touch_mode_change_listeners
            .get(&map_identity_key(&observer))
            .cloned(),
        Some(vec![touch_mode_listener.clone()])
    );
    assert_eq!(
        program.invoke_builtin_method(
            &remove_listener,
            Some(&test_invocation(vec![
                observer.clone(),
                global_layout_listener
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert!(!program
        .runtime
        .borrow()
        .view_tree_global_layout_listeners
        .contains_key(&map_identity_key(&observer)));
    assert_eq!(
        program.invoke_builtin_method(
            &remove_draw_listener,
            Some(&test_invocation(vec![observer.clone(), draw_listener]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert!(!program
        .runtime
        .borrow()
        .view_tree_draw_listeners
        .contains_key(&map_identity_key(&observer)));
    assert_eq!(
        program.invoke_builtin_method(
            &remove_scroll_listener,
            Some(&test_invocation(vec![observer.clone(), scroll_listener]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert!(!program
        .runtime
        .borrow()
        .view_tree_scroll_changed_listeners
        .contains_key(&map_identity_key(&observer)));
    assert_eq!(
        program.invoke_builtin_method(
            &remove_window_focus_listener,
            Some(&test_invocation(vec![
                observer.clone(),
                window_focus_listener
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert!(!program
        .runtime
        .borrow()
        .view_tree_window_focus_change_listeners
        .contains_key(&map_identity_key(&observer)));
    assert_eq!(
        program.invoke_builtin_method(
            &remove_touch_mode_listener,
            Some(&test_invocation(vec![
                observer.clone(),
                touch_mode_listener
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert!(!program
        .runtime
        .borrow()
        .view_tree_touch_mode_change_listeners
        .contains_key(&map_identity_key(&observer)));
}

#[test]
fn builtin_view_attach_state_listener_methods_are_bootstrapped() {
    let program = DexProgram::new(Vec::new());
    let add_listener = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "addOnAttachStateChangeListener".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/view/View$OnAttachStateChangeListener;".to_owned()],
        },
    };
    let remove_listener = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "removeOnAttachStateChangeListener".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/view/View$OnAttachStateChangeListener;".to_owned()],
        },
    };
    let is_attached = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "isAttachedToWindow".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let has_window_focus = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "hasWindowFocus".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let view = DexValue::Reference(std::rc::Rc::from(
        "Lcom/android/internal/policy/DecorView;.decor",
    ));
    let detached_view = DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:detached"));
    let listener = DexValue::Reference(std::rc::Rc::from(
        "Landroid/view/View$OnAttachStateChangeListener;:test",
    ));

    assert_eq!(
        program.invoke_builtin_method(
            &add_listener,
            Some(&test_invocation(vec![view.clone(), listener.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .view_attach_state_listeners
            .get(&map_identity_key(&view))
            .cloned(),
        Some(vec![listener.clone()])
    );
    assert_eq!(
        program.invoke_builtin_method(&is_attached, Some(&test_invocation(vec![view.clone()]))),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &has_window_focus,
            Some(&test_invocation(vec![view.clone()]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &has_window_focus,
            Some(&test_invocation(vec![detached_view]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &remove_listener,
            Some(&test_invocation(vec![view.clone(), listener]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert!(!program
        .runtime
        .borrow()
        .view_attach_state_listeners
        .contains_key(&map_identity_key(&view)));
}

#[test]
fn view_attach_listener_dispatch_errors_when_callback_is_missing() {
    let program = DexProgram::new(Vec::new());
    let add_listener = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "addOnAttachStateChangeListener".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/view/View$OnAttachStateChangeListener;".to_owned()],
        },
    };
    let view = DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:compose"));
    let listener = DexValue::Object(DexObjectRef {
        id: 7,
        class_descriptor: std::rc::Rc::from("Lexample/AttachListener;"),
    });

    assert_eq!(
        program.invoke_builtin_method(
            &add_listener,
            Some(&test_invocation(vec![view.clone(), listener]))
        ),
        Some(DexExecutionResult::Void)
    );
    let error = program
        .dispatch_view_attached_to_window(&view, 0)
        .expect_err("missing attach listener method should fail explicitly");

    assert!(
        error
            .to_string()
            .contains("View attach listener method was not found"),
        "unexpected error: {error}"
    );
    assert!(
        error.to_string().contains("next missing primitive:"),
        "unexpected error: {error}"
    );
}

#[test]
fn builtin_viewgroup_on_attached_to_window_super_is_noop() {
    let program = DexProgram::new(Vec::new());
    let on_attached = DexMethodId {
        class_descriptor: "Landroid/view/ViewGroup;".to_owned(),
        name: "onAttachedToWindow".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };

    assert_eq!(
        program.invoke_builtin_method(
            &on_attached,
            Some(&test_invocation(vec![DexValue::Reference(
                std::rc::Rc::from("Landroid/view/ViewGroup;:root")
            )]))
        ),
        Some(DexExecutionResult::Void)
    );
}

#[test]
fn builtin_view_get_handler_returns_main_handler_for_attached_view() {
    let program = DexProgram::new(Vec::new());
    let set_content_view = DexMethodId {
        class_descriptor: "Landroid/app/Activity;".to_owned(),
        name: "setContentView".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/view/View;".to_owned()],
        },
    };
    let get_handler = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "getHandler".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/os/Handler;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let activity = DexValue::Reference(std::rc::Rc::from("Landroid/app/Activity;:test"));
    let view = DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:compose"));

    assert_eq!(
        program.invoke_builtin_method(&get_handler, Some(&test_invocation(vec![view.clone()]))),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_content_view,
            Some(&test_invocation(vec![activity, view.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );

    let result = program.invoke_builtin_method(&get_handler, Some(&test_invocation(vec![view])));
    let Some(DexExecutionResult::Value(handler)) = result else {
        panic!("attached View.getHandler should return a handler");
    };
    assert_eq!(
        value_class_descriptor(&handler),
        Some("Landroid/os/Handler;")
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .handler_loopers
            .get(&map_identity_key(&handler))
            .cloned(),
        Some(DexValue::Reference(std::rc::Rc::from(
            "Landroid/os/Looper;:main"
        )))
    );
}

#[test]
fn view_tree_draw_listener_dispatch_errors_when_callback_is_missing() {
    let program = DexProgram::new(Vec::new());
    let add_listener = DexMethodId {
        class_descriptor: "Landroid/view/ViewTreeObserver;".to_owned(),
        name: "addOnDrawListener".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/view/ViewTreeObserver$OnDrawListener;".to_owned()],
        },
    };
    let observer = DexValue::Reference(std::rc::Rc::from(
        "Landroid/view/ViewTreeObserver;:reference:Lcom/android/internal/policy/DecorView;.decor",
    ));
    let listener = DexValue::Object(DexObjectRef {
        id: 9,
        class_descriptor: std::rc::Rc::from("Lexample/DrawListener;"),
    });

    assert_eq!(
        program.invoke_builtin_method(
            &add_listener,
            Some(&test_invocation(vec![observer, listener]))
        ),
        Some(DexExecutionResult::Void)
    );
    let error = program
        .dispatch_view_tree_draw_listeners(0)
        .expect_err("missing draw listener method should fail explicitly");

    assert!(
        error
            .to_string()
            .contains("ViewTreeObserver draw listener method was not found"),
        "unexpected error: {error}"
    );
    assert!(
        error.to_string().contains("next missing primitive:"),
        "unexpected error: {error}"
    );
}

#[test]
fn view_tree_global_layout_listener_dispatch_errors_when_callback_is_missing() {
    let program = DexProgram::new(Vec::new());
    let add_listener = DexMethodId {
        class_descriptor: "Landroid/view/ViewTreeObserver;".to_owned(),
        name: "addOnGlobalLayoutListener".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/view/ViewTreeObserver$OnGlobalLayoutListener;".to_owned()],
        },
    };
    let observer = DexValue::Reference(std::rc::Rc::from(
        "Landroid/view/ViewTreeObserver;:reference:Lcom/android/internal/policy/DecorView;.decor",
    ));
    let listener = DexValue::Object(DexObjectRef {
        id: 10,
        class_descriptor: std::rc::Rc::from("Lexample/GlobalLayoutListener;"),
    });

    assert_eq!(
        program.invoke_builtin_method(
            &add_listener,
            Some(&test_invocation(vec![observer, listener]))
        ),
        Some(DexExecutionResult::Void)
    );
    let error = program
        .dispatch_view_tree_global_layout_listeners(0)
        .expect_err("missing global layout listener method should fail explicitly");

    assert!(
        error
            .to_string()
            .contains("ViewTreeObserver global layout listener method was not found"),
        "unexpected error: {error}"
    );
    assert!(
        error.to_string().contains("next missing primitive:"),
        "unexpected error: {error}"
    );
}

#[test]
fn builtin_view_get_window_token_returns_stable_binder_reference() {
    let program = DexProgram::new(Vec::new());
    let get_window_token = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "getWindowToken".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/os/IBinder;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get_application_window_token = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "getApplicationWindowToken".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/os/IBinder;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let view = DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:id/2131296428"));
    let expected = DexExecutionResult::Value(DexValue::Reference(std::rc::Rc::from(
        "Landroid/os/IBinder;:window-token:reference:Landroid/view/View;:id/2131296428",
    )));

    assert_eq!(
        program.invoke_builtin_method(
            &get_window_token,
            Some(&test_invocation(vec![view.clone()]))
        ),
        Some(expected.clone())
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_application_window_token,
            Some(&test_invocation(vec![view]))
        ),
        Some(expected)
    );
}

#[test]
fn builtin_view_find_on_back_invoked_dispatcher_returns_bootstrap_dispatcher() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "findOnBackInvokedDispatcher".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/window/OnBackInvokedDispatcher;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let view = DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:id/2131296428"));

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![view]))),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Landroid/window/OnBackInvokedDispatcher;.bootstrap")
        )))
    );
}

#[test]
fn builtin_view_measure_spec_packs_mode_and_size() {
    let program = DexProgram::new(Vec::new());
    let make_measure_spec = DexMethodId {
        class_descriptor: "Landroid/view/View$MeasureSpec;".to_owned(),
        name: "makeMeasureSpec".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "III".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["I".to_owned(), "I".to_owned()],
        },
    };
    let get_mode = DexMethodId {
        class_descriptor: "Landroid/view/View$MeasureSpec;".to_owned(),
        name: "getMode".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "II".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let get_size = DexMethodId {
        class_descriptor: "Landroid/view/View$MeasureSpec;".to_owned(),
        name: "getSize".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "II".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let get_measured_width = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "getMeasuredWidth".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get_measured_height = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "getMeasuredHeight".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let at_most = 0x8000_0000u32 as i32;
    let spec = at_most | 320;
    let view = DexValue::Reference(std::rc::Rc::from("Landroid/view/View;.measured"));

    assert_eq!(
        program.invoke_builtin_method(
            &make_measure_spec,
            Some(&test_invocation(vec![
                DexValue::Int(320),
                DexValue::Int(at_most)
            ]))
        ),
        Some(DexExecutionResult::Int(spec))
    );
    assert_eq!(
        program.invoke_builtin_method(&get_mode, Some(&test_invocation(vec![DexValue::Int(spec)]))),
        Some(DexExecutionResult::Int(at_most))
    );
    assert_eq!(
        program.invoke_builtin_method(&get_size, Some(&test_invocation(vec![DexValue::Int(spec)]))),
        Some(DexExecutionResult::Int(320))
    );
    // cite: frameworks-base View.java#getDefaultSize
    program
        .measure_view(&view, spec, at_most | 180, 0)
        .expect("View.measure without an onMeasure override should succeed");
    assert_eq!(
        program.invoke_builtin_method(
            &get_measured_width,
            Some(&test_invocation(vec![view.clone()]))
        ),
        Some(DexExecutionResult::Int(320))
    );
    assert_eq!(
        program.invoke_builtin_method(&get_measured_height, Some(&test_invocation(vec![view]))),
        Some(DexExecutionResult::Int(180))
    );
}

#[test]
fn builtin_view_get_resources_returns_bootstrap_resources() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
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
fn builtin_view_get_context_returns_bootstrap_context() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "getContext".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/content/Context;".to_owned(),
            parameters: Vec::new(),
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&method, None),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Landroid/content/Context;.bootstrap")
        )))
    );
}

#[test]
fn builtin_view_get_context_returns_active_activity_context() {
    let program = DexProgram::new(Vec::new());
    let activity = DexValue::Object(DexObjectRef {
        id: 1,
        class_descriptor: std::rc::Rc::from("Lexample/MainActivity;"),
    });
    let method = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "getContext".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/content/Context;".to_owned(),
            parameters: Vec::new(),
        },
    };

    program.write_activity_receiver(activity.clone());

    assert_eq!(
        program.invoke_builtin_method(&method, None),
        Some(DexExecutionResult::Value(activity))
    );
}

#[test]
fn builtin_view_save_attribute_data_for_styleable_returns_void() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "saveAttributeDataForStyleable".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VLLLLII".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "Landroid/content/Context;".to_owned(),
                "[I".to_owned(),
                "Landroid/util/AttributeSet;".to_owned(),
                "Landroid/content/res/TypedArray;".to_owned(),
                "I".to_owned(),
                "I".to_owned(),
            ],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::Reference(std::rc::Rc::from("Landroid/view/View;.styled")),
                DexValue::Reference(std::rc::Rc::from("Landroid/content/Context;.bootstrap")),
                DexValue::Reference(std::rc::Rc::from("[I:styleable")),
                DexValue::Int(0),
                DexValue::Reference(std::rc::Rc::from(
                    "Landroid/content/res/TypedArray;:styleable"
                )),
                DexValue::Int(2130969837),
                DexValue::Int(0),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
}

#[test]
fn builtin_view_configuration_returns_bootstrap_values() {
    let program = DexProgram::new(Vec::new());
    let get = DexMethodId {
        class_descriptor: "Landroid/view/ViewConfiguration;".to_owned(),
        name: "get".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Landroid/view/ViewConfiguration;".to_owned(),
            parameters: vec!["Landroid/content/Context;".to_owned()],
        },
    };
    let touch_slop = DexMethodId {
        class_descriptor: "Landroid/view/ViewConfiguration;".to_owned(),
        name: "getScaledTouchSlop".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let tap_timeout = DexMethodId {
        class_descriptor: "Landroid/view/ViewConfiguration;".to_owned(),
        name: "getTapTimeout".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let long_press_timeout = DexMethodId {
        class_descriptor: "Landroid/view/ViewConfiguration;".to_owned(),
        name: "getLongPressTimeout".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let show_menu_shortcuts = DexMethodId {
        class_descriptor: "Landroid/view/ViewConfiguration;".to_owned(),
        name: "shouldShowMenuShortcutsWhenKeyboardPresent".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let scroll_friction = DexMethodId {
        class_descriptor: "Landroid/view/ViewConfiguration;".to_owned(),
        name: "getScrollFriction".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "F".to_owned(),
            return_type: "F".to_owned(),
            parameters: Vec::new(),
        },
    };
    let horizontal_scroll_factor = DexMethodId {
        class_descriptor: "Landroid/view/ViewConfiguration;".to_owned(),
        name: "getScaledHorizontalScrollFactor".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "F".to_owned(),
            return_type: "F".to_owned(),
            parameters: Vec::new(),
        },
    };
    let vertical_scroll_factor = DexMethodId {
        class_descriptor: "Landroid/view/ViewConfiguration;".to_owned(),
        name: "getScaledVerticalScrollFactor".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "F".to_owned(),
            return_type: "F".to_owned(),
            parameters: Vec::new(),
        },
    };
    let context = DexValue::Reference(std::rc::Rc::from("Landroid/content/Context;.bootstrap"));

    assert_eq!(
        program.invoke_builtin_method(&get, Some(&test_invocation(vec![context]))),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Landroid/view/ViewConfiguration;.bootstrap")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&touch_slop, None),
        Some(DexExecutionResult::Int(8))
    );
    assert_eq!(
        program.invoke_builtin_method(&tap_timeout, None),
        Some(DexExecutionResult::Int(100))
    );
    assert_eq!(
        program.invoke_builtin_method(&long_press_timeout, None),
        Some(DexExecutionResult::Int(400))
    );
    assert_eq!(
        program.invoke_builtin_method(&show_menu_shortcuts, None),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&scroll_friction, None),
        Some(DexExecutionResult::Value(DexValue::Float(0.015)))
    );
    assert_eq!(
        program.invoke_builtin_method(&horizontal_scroll_factor, None),
        Some(DexExecutionResult::Value(DexValue::Float(64.0)))
    );
    assert_eq!(
        program.invoke_builtin_method(&vertical_scroll_factor, None),
        Some(DexExecutionResult::Value(DexValue::Float(64.0)))
    );
}

#[test]
fn builtin_key_character_map_load_returns_device_scoped_map() {
    let program = DexProgram::new(Vec::new());
    let load = DexMethodId {
        class_descriptor: "Landroid/view/KeyCharacterMap;".to_owned(),
        name: "load".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LI".to_owned(),
            return_type: "Landroid/view/KeyCharacterMap;".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let get_keyboard_type = DexMethodId {
        class_descriptor: "Landroid/view/KeyCharacterMap;".to_owned(),
        name: "getKeyboardType".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };

    let Some(DexExecutionResult::Value(key_character_map)) =
        program.invoke_builtin_method(&load, Some(&test_invocation(vec![DexValue::Int(-1)])))
    else {
        panic!("KeyCharacterMap.load should return a map");
    };
    assert_eq!(
        key_character_map,
        DexValue::Reference(std::rc::Rc::from(
            "Landroid/view/KeyCharacterMap;:device:-1"
        ))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_keyboard_type,
            Some(&test_invocation(vec![key_character_map]))
        ),
        Some(DexExecutionResult::Int(3))
    );
}

#[test]
fn builtin_window_insets_builder_builds_bootstrap_insets() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Landroid/view/WindowInsets$Builder;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let build = DexMethodId {
        class_descriptor: "Landroid/view/WindowInsets$Builder;".to_owned(),
        name: "build".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/view/WindowInsets;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set_system_window_insets = DexMethodId {
        class_descriptor: "Landroid/view/WindowInsets$Builder;".to_owned(),
        name: "setSystemWindowInsets".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Landroid/view/WindowInsets$Builder;".to_owned(),
            parameters: vec!["Landroid/graphics/Insets;".to_owned()],
        },
    };
    let builder = DexValue::Reference(std::rc::Rc::from(
        "Landroid/view/WindowInsets$Builder;:main",
    ));
    let insets = DexValue::Reference(std::rc::Rc::from("Landroid/graphics/Insets;:system"));

    assert_eq!(
        program.invoke_builtin_method(&constructor, None),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_system_window_insets,
            Some(&test_invocation(vec![builder.clone(), insets.clone()]))
        ),
        Some(DexExecutionResult::Value(builder.clone()))
    );
    assert_eq!(
        program.read_stored_instance_field(
            &builder,
            &window_insets_builder_field("systemWindowInsets")
        ),
        Some(insets)
    );
    assert_eq!(
        program.invoke_builtin_method(&build, Some(&test_invocation(vec![builder.clone()]))),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from(format!(
                "Landroid/view/WindowInsets;:builder:{}",
                map_identity_key(&builder)
            ))
        )))
    );
}

#[test]
fn builtin_window_insets_type_returns_android_masks() {
    let program = DexProgram::new(Vec::new());

    for (name, expected) in [
        ("statusBars", 1),
        ("navigationBars", 2),
        ("captionBar", 4),
        ("ime", 8),
        ("systemGestures", 16),
        ("mandatorySystemGestures", 32),
        ("tappableElement", 64),
        ("displayCutout", 128),
        ("systemOverlays", 512),
        ("systemBars", 7),
        ("all", -1),
    ] {
        let method = DexMethodId {
            class_descriptor: "Landroid/view/WindowInsets$Type;".to_owned(),
            name: name.to_owned(),
            proto: crate::dex::DexProto {
                shorty: "I".to_owned(),
                return_type: "I".to_owned(),
                parameters: Vec::new(),
            },
        };

        assert_eq!(
            program.invoke_builtin_method(&method, None),
            Some(DexExecutionResult::Int(expected))
        );
    }
}

#[test]
fn builtin_window_insets_visibility_returns_default_root_state() {
    let program = DexProgram::new(Vec::new());
    let is_visible = DexMethodId {
        class_descriptor: "Landroid/view/WindowInsets;".to_owned(),
        name: "isVisible".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZI".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let insets = DexValue::Reference(std::rc::Rc::from("Landroid/view/WindowInsets;:root:decor"));

    assert_eq!(
        program.invoke_builtin_method(
            &is_visible,
            Some(&test_invocation(vec![insets.clone(), DexValue::Int(7)]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &is_visible,
            Some(&test_invocation(vec![insets, DexValue::Int(8)]))
        ),
        Some(DexExecutionResult::Int(0))
    );
}

#[test]
fn builtin_gesture_detector_startup_methods_return_defaults() {
    let program = DexProgram::new(Vec::new());
    let listener_constructor = DexMethodId {
        class_descriptor: "Landroid/view/GestureDetector$SimpleOnGestureListener;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let detector_constructor = DexMethodId {
        class_descriptor: "Landroid/view/GestureDetector;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VLL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "Landroid/content/Context;".to_owned(),
                "Landroid/view/GestureDetector$OnGestureListener;".to_owned(),
            ],
        },
    };
    let set_long_press = DexMethodId {
        class_descriptor: "Landroid/view/GestureDetector;".to_owned(),
        name: "setIsLongpressEnabled".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VZ".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Z".to_owned()],
        },
    };
    let on_touch_event = DexMethodId {
        class_descriptor: "Landroid/view/GestureDetector;".to_owned(),
        name: "onTouchEvent".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Landroid/view/MotionEvent;".to_owned()],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&listener_constructor, None),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&detector_constructor, None),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&set_long_press, None),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&on_touch_event, None),
        Some(DexExecutionResult::Int(0))
    );
}

#[test]
fn builtin_view_is_laid_out_returns_false() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "isLaidOut".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let is_showing_layout_bounds = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "isShowingLayoutBounds".to_owned(),
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
    assert_eq!(
        program.invoke_builtin_method(&is_showing_layout_bounds, None),
        Some(DexExecutionResult::Int(0))
    );
}

#[test]
fn builtin_view_layout_stores_bounds_and_dimensions() {
    let program = DexProgram::new(Vec::new());
    let layout = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "layout".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VIIII".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "I".to_owned(),
                "I".to_owned(),
                "I".to_owned(),
                "I".to_owned(),
            ],
        },
    };
    let is_laid_out = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "isLaidOut".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get_location_on_screen = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "getLocationOnScreen".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V[".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["[I".to_owned()],
        },
    };
    let view = DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:geometry"));
    let location = program
        .allocate_array("[I".to_owned(), 2)
        .expect("location array should allocate");

    assert_eq!(
        program.invoke_builtin_method(
            &layout,
            Some(&test_invocation(vec![
                view.clone(),
                DexValue::Int(10),
                DexValue::Int(20),
                DexValue::Int(110),
                DexValue::Int(70),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&is_laid_out, Some(&test_invocation(vec![view.clone()]))),
        Some(DexExecutionResult::Int(1))
    );

    for (name, expected) in [
        ("getLeft", 10),
        ("getTop", 20),
        ("getRight", 110),
        ("getBottom", 70),
        ("getWidth", 100),
        ("getHeight", 50),
    ] {
        let method = DexMethodId {
            class_descriptor: "Landroid/view/View;".to_owned(),
            name: name.to_owned(),
            proto: crate::dex::DexProto {
                shorty: "I".to_owned(),
                return_type: "I".to_owned(),
                parameters: Vec::new(),
            },
        };

        assert_eq!(
            program.invoke_builtin_method(&method, Some(&test_invocation(vec![view.clone()]))),
            Some(DexExecutionResult::Int(expected))
        );
    }
    for (name, value) in [
        ("setLeft", 15),
        ("setTop", 25),
        ("setRight", 115),
        ("setBottom", 75),
    ] {
        let method = DexMethodId {
            class_descriptor: "Landroid/view/View;".to_owned(),
            name: name.to_owned(),
            proto: crate::dex::DexProto {
                shorty: "VI".to_owned(),
                return_type: "V".to_owned(),
                parameters: vec!["I".to_owned()],
            },
        };

        assert_eq!(
            program.invoke_builtin_method(
                &method,
                Some(&test_invocation(vec![view.clone(), DexValue::Int(value)]))
            ),
            Some(DexExecutionResult::Void)
        );
    }
    for (name, expected) in [
        ("getLeft", 15),
        ("getTop", 25),
        ("getRight", 115),
        ("getBottom", 75),
        ("getWidth", 100),
        ("getHeight", 50),
    ] {
        let method = DexMethodId {
            class_descriptor: "Landroid/view/View;".to_owned(),
            name: name.to_owned(),
            proto: crate::dex::DexProto {
                shorty: "I".to_owned(),
                return_type: "I".to_owned(),
                parameters: Vec::new(),
            },
        };

        assert_eq!(
            program.invoke_builtin_method(&method, Some(&test_invocation(vec![view.clone()]))),
            Some(DexExecutionResult::Int(expected))
        );
    }
    assert_eq!(
        program.invoke_builtin_method(
            &get_location_on_screen,
            Some(&test_invocation(vec![view, location.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    // cite: frameworks-base View.java#getLocationOnScreen, getLocationInWindow, transformFromViewToWindowSpace
    assert_eq!(
        program.read_array_value(&location, 0),
        Some(DexValue::Int(0))
    );
    assert_eq!(
        program.read_array_value(&location, 1),
        Some(DexValue::Int(0))
    );
}

#[test]
fn builtin_view_window_visible_display_frame_uses_runtime_viewport() {
    let program = DexProgram::new(Vec::new());
    let get_window_visible_display_frame = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "getWindowVisibleDisplayFrame".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/graphics/Rect;".to_owned()],
        },
    };
    let view = DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:pre-layout"));
    let rect = DexValue::Reference(std::rc::Rc::from("Landroid/graphics/Rect;:visible-frame"));

    program
        .dispatch_runtime_view_frames(420, 760, 0)
        .expect("runtime viewport should be accepted");
    assert_eq!(
        program.invoke_builtin_method(
            &get_window_visible_display_frame,
            Some(&test_invocation(vec![view, rect.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );

    for (name, expected) in [("left", 0), ("top", 0), ("right", 420), ("bottom", 760)] {
        assert_eq!(
            program.read_stored_instance_field(&rect, &rect_int_field(name)),
            Some(DexValue::Int(expected))
        );
    }
}

fn write_test_view_bounds(
    program: &DexProgram,
    view: &DexValue,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
) {
    for (name, value) in [
        ("left", left),
        ("top", top),
        ("right", right),
        ("bottom", bottom),
    ] {
        assert!(program.write_stored_instance_field(
            view,
            &view_bounds_int_field(name),
            DexValue::Int(value),
        ));
    }
}

// cite: frameworks-base View.java#getLocationOnScreen, getLocationInWindow, transformFromViewToWindowSpace
#[test]
fn aosp_view_location_on_screen_uses_attach_origin_not_requested_window_layout_offsets() {
    let program = DexProgram::new(Vec::new());
    let root = DexValue::Reference(std::rc::Rc::from("Landroid/view/ViewGroup;:location-root"));
    let parent = DexValue::Reference(std::rc::Rc::from(
        "Landroid/widget/HorizontalScrollView;:location-parent",
    ));
    let child = DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:location-child"));
    let window_params = DexValue::Reference(std::rc::Rc::from(
        "Landroid/view/WindowManager$LayoutParams;:location-window-params",
    ));

    write_test_view_bounds(&program, &root, 3, 4, 103, 104);
    write_test_view_bounds(&program, &parent, 10, 20, 90, 90);
    write_test_view_bounds(&program, &child, 5, 6, 45, 36);
    assert!(program.write_stored_instance_field(
        &parent,
        &horizontal_scroll_view_scroll_x_field(),
        DexValue::Int(7),
    ));
    assert!(program.write_stored_instance_field(&parent, &view_scroll_y_field(), DexValue::Int(8),));
    assert!(program.write_stored_instance_field(
        &window_params,
        &window_manager_layout_params_int_field("x"),
        DexValue::Int(200),
    ));
    assert!(program.write_stored_instance_field(
        &window_params,
        &window_manager_layout_params_int_field("y"),
        DexValue::Int(300),
    ));
    assert!(program.write_stored_instance_field(&root, &view_layout_params_field(), window_params,));
    {
        let mut runtime = program.runtime.borrow_mut();
        runtime.runtime_view_roots.push(DexRuntimeViewRoot {
            view_key: map_identity_key(&root),
            class_descriptor: "Landroid/view/ViewGroup;".to_owned(),
        });
        runtime
            .view_parents
            .insert(map_identity_key(&child), parent.clone());
        runtime.view_parents.insert(map_identity_key(&parent), root);
    }

    assert_eq!(program.view_location_on_screen(&child), Some((11, 22)));
}

#[test]
fn aosp_view_update_display_list_if_dirty_replays_clean_render_nodes() {
    let program = DexProgram::new(Vec::new());
    let root = DexValue::Reference(std::rc::Rc::from("Landroid/view/ViewGroup;:retained-root"));
    let child = DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:retained-child"));
    let first_canvas = DexValue::Reference(std::rc::Rc::from("Landroid/graphics/Canvas;:first"));
    let second_canvas = DexValue::Reference(std::rc::Rc::from("Landroid/graphics/Canvas;:second"));

    write_test_view_bounds(&program, &root, 0, 0, 100, 100);
    write_test_view_bounds(&program, &child, 10, 20, 80, 90);
    {
        let mut runtime = program.runtime.borrow_mut();
        runtime
            .view_group_children
            .insert(map_identity_key(&root), vec![child.clone()]);
        runtime
            .view_parents
            .insert(map_identity_key(&child), root.clone());
    }

    program
        .dispatch_runtime_view_draw(&root, &first_canvas, 0)
        .expect("AOSP View.updateDisplayListIfDirty first draw should record display lists");
    let root_key = map_identity_key(&root);
    let child_key = map_identity_key(&child);
    let root_render_node = DexValue::Reference(std::rc::Rc::from(format!(
        "Landroid/graphics/RenderNode;:view:{root_key}"
    )));
    let child_render_node = DexValue::Reference(std::rc::Rc::from(format!(
        "Landroid/graphics/RenderNode;:view:{child_key}"
    )));
    {
        let runtime = program.runtime.borrow();
        assert_eq!(
            runtime.view_render_node_generations.get(&root_key),
            Some(&1)
        );
        assert_eq!(
            runtime.view_render_node_generations.get(&child_key),
            Some(&1)
        );
        assert_eq!(
            runtime
                .render_node_draw_ops
                .get(&map_identity_key(&root_render_node)),
            Some(&vec![CanvasDrawOp::RenderNode {
                render_node: child_render_node.clone()
            }])
        );
    }

    program
        .dispatch_runtime_view_draw(&root, &second_canvas, 0)
        .expect("AOSP View.draw should replay a clean RenderNode display list");
    let runtime = program.runtime.borrow();
    assert_eq!(
        runtime.view_render_node_generations.get(&root_key),
        Some(&1)
    );
    assert_eq!(
        runtime.view_render_node_generations.get(&child_key),
        Some(&1)
    );
    assert_eq!(
        runtime
            .canvas_draw_ops
            .get(&map_identity_key(&second_canvas)),
        Some(&vec![CanvasDrawOp::RenderNode {
            render_node: root_render_node
        }])
    );
}

#[test]
fn aosp_view_set_alpha_and_elevation_update_render_node_without_rerecording_display_list() {
    let program = DexProgram::new(Vec::new());
    let set_alpha = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "setAlpha".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VF".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["F".to_owned()],
        },
    };
    let set_elevation = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "setElevation".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VF".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["F".to_owned()],
        },
    };
    let root = DexValue::Reference(std::rc::Rc::from(
        "Landroid/view/ViewGroup;:property-retained-root",
    ));
    let child = DexValue::Reference(std::rc::Rc::from(
        "Landroid/view/View;:property-retained-child",
    ));
    let first_canvas = DexValue::Reference(std::rc::Rc::from(
        "Landroid/graphics/Canvas;:property-retained-first",
    ));
    let second_canvas = DexValue::Reference(std::rc::Rc::from(
        "Landroid/graphics/Canvas;:property-retained-second",
    ));

    write_test_view_bounds(&program, &root, 0, 0, 100, 100);
    write_test_view_bounds(&program, &child, 10, 20, 80, 90);
    {
        let mut runtime = program.runtime.borrow_mut();
        runtime
            .view_group_children
            .insert(map_identity_key(&root), vec![child.clone()]);
        runtime
            .view_parents
            .insert(map_identity_key(&child), root.clone());
    }

    program
        .dispatch_runtime_view_draw(&root, &first_canvas, 0)
        .expect("AOSP View.updateDisplayListIfDirty initial draw should record display lists");
    let root_key = map_identity_key(&root);
    let child_key = map_identity_key(&child);
    let root_render_node = DexValue::Reference(std::rc::Rc::from(format!(
        "Landroid/graphics/RenderNode;:view:{root_key}"
    )));
    let child_render_node = DexValue::Reference(std::rc::Rc::from(format!(
        "Landroid/graphics/RenderNode;:view:{child_key}"
    )));
    assert_eq!(
        program.invoke_builtin_method(
            &set_alpha,
            Some(&test_invocation(vec![child.clone(), DexValue::Float(0.5)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_elevation,
            Some(&test_invocation(vec![child.clone(), DexValue::Float(12.0)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&child_render_node, &render_node_alpha_field()),
        Some(DexValue::Float(0.5))
    );
    assert_eq!(
        program.read_stored_instance_field(&child_render_node, &render_node_elevation_field()),
        Some(DexValue::Float(12.0))
    );
    {
        let runtime = program.runtime.borrow();
        assert!(runtime.has_traversal_request());
        assert!(!runtime.view_render_node_dirty.contains(&root_key));
        assert!(!runtime.view_render_node_dirty.contains(&child_key));
    }

    program
        .dispatch_runtime_view_draw(&root, &second_canvas, 0)
        .expect("AOSP View.invalidateViewProperty should replay retained display lists");
    let runtime = program.runtime.borrow();
    assert_eq!(
        runtime.view_render_node_generations.get(&root_key),
        Some(&1)
    );
    assert_eq!(
        runtime.view_render_node_generations.get(&child_key),
        Some(&1)
    );
    assert_eq!(
        runtime
            .canvas_draw_ops
            .get(&map_identity_key(&second_canvas)),
        Some(&vec![CanvasDrawOp::RenderNode {
            render_node: root_render_node
        }])
    );
}

#[test]
fn androidx_render_node_layer_trigger_repaint_replays_clean_view_display_list() {
    let program = DexProgram::new(Vec::new());
    let on_descendant_invalidated = DexMethodId {
        class_descriptor: "Landroid/view/ViewParent;".to_owned(),
        name: "onDescendantInvalidated".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VLL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "Landroid/view/View;".to_owned(),
                "Landroid/view/View;".to_owned(),
            ],
        },
    };
    let root = DexValue::Reference(std::rc::Rc::from(
        "Landroid/view/ViewGroup;:compose-retained-root",
    ));
    let child = DexValue::Reference(std::rc::Rc::from(
        "Landroid/view/View;:compose-retained-child",
    ));
    let first_canvas = DexValue::Reference(std::rc::Rc::from(
        "Landroid/graphics/Canvas;:compose-retained-first",
    ));

    write_test_view_bounds(&program, &root, 0, 0, 100, 100);
    write_test_view_bounds(&program, &child, 0, 0, 100, 100);
    {
        let mut runtime = program.runtime.borrow_mut();
        runtime
            .view_group_children
            .insert(map_identity_key(&root), vec![child.clone()]);
        runtime
            .view_parents
            .insert(map_identity_key(&child), root.clone());
        runtime.runtime_view_roots.push(DexRuntimeViewRoot {
            view_key: map_identity_key(&root),
            class_descriptor: "Landroid/view/ViewGroup;".to_owned(),
        });
    }

    program
        .dispatch_runtime_view_draw(&root, &first_canvas, 0)
        .expect("AOSP View.updateDisplayListIfDirty initial draw should record display lists");
    let root_key = map_identity_key(&root);
    let child_key = map_identity_key(&child);
    let root_render_node = DexValue::Reference(std::rc::Rc::from(format!(
        "Landroid/graphics/RenderNode;:view:{root_key}"
    )));
    let root_target_render_node = DexValue::Reference(std::rc::Rc::from(format!(
        "Landroid/graphics/RenderNode;:root:{root_key}"
    )));
    assert_eq!(
        program
            .runtime
            .borrow()
            .view_render_node_generations
            .get(&root_key),
        Some(&1)
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .view_render_node_generations
            .get(&child_key),
        Some(&1)
    );

    assert_eq!(
        program.invoke_builtin_method(
            &on_descendant_invalidated,
            Some(&test_invocation(vec![
                root.clone(),
                child.clone(),
                child.clone()
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    {
        let runtime = program.runtime.borrow();
        assert!(runtime.has_traversal_request());
        assert!(!runtime.view_render_node_dirty.contains(&root_key));
        assert!(!runtime.view_render_node_dirty.contains(&child_key));
    }

    assert_eq!(
        program
            .dispatch_runtime_view_frames(420, 760, 0)
            .expect("AndroidX RenderNodeLayer.triggerRepaint should schedule a draw traversal"),
        1
    );
    let runtime = program.runtime.borrow();
    assert_eq!(
        runtime.view_render_node_generations.get(&root_key),
        Some(&1)
    );
    assert_eq!(
        runtime.view_render_node_generations.get(&child_key),
        Some(&1)
    );
    assert_eq!(runtime.runtime_canvas_frames.len(), 1);
    assert_eq!(
        runtime
            .canvas_draw_ops
            .get(&runtime.runtime_canvas_frames[0].canvas_key),
        Some(&vec![CanvasDrawOp::RenderNode {
            render_node: root_target_render_node.clone()
        }])
    );
    assert_eq!(
        runtime
            .render_node_draw_ops
            .get(&map_identity_key(&root_target_render_node)),
        Some(&vec![CanvasDrawOp::RenderNode {
            render_node: root_render_node
        }])
    );
}

#[test]
fn aosp_threaded_renderer_root_display_list_replays_clean_view_render_node() {
    let program = DexProgram::new(Vec::new());
    let root = DexValue::Reference(std::rc::Rc::from("Landroid/view/ViewGroup;:threaded-root"));
    let child = DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:threaded-child"));

    write_test_view_bounds(&program, &root, 0, 0, 100, 100);
    write_test_view_bounds(&program, &child, 10, 20, 80, 90);
    {
        let mut runtime = program.runtime.borrow_mut();
        runtime
            .view_group_children
            .insert(map_identity_key(&root), vec![child.clone()]);
        runtime
            .view_parents
            .insert(map_identity_key(&child), root.clone());
        runtime.runtime_view_roots.push(DexRuntimeViewRoot {
            view_key: map_identity_key(&root),
            class_descriptor: "Landroid/view/ViewGroup;".to_owned(),
        });
        runtime.request_draw_traversal();
    }

    assert_eq!(
        program
            .dispatch_runtime_view_frames(420, 760, 0)
            .expect("AOSP ThreadedRenderer should record a root RenderNode target"),
        1
    );
    let root_key = map_identity_key(&root);
    let child_key = map_identity_key(&child);
    let root_view_render_node = DexValue::Reference(std::rc::Rc::from(format!(
        "Landroid/graphics/RenderNode;:view:{root_key}"
    )));
    let child_render_node = DexValue::Reference(std::rc::Rc::from(format!(
        "Landroid/graphics/RenderNode;:view:{child_key}"
    )));
    let root_target_render_node = DexValue::Reference(std::rc::Rc::from(format!(
        "Landroid/graphics/RenderNode;:root:{root_key}"
    )));
    {
        let runtime = program.runtime.borrow();
        assert_eq!(
            runtime.view_render_node_generations.get(&root_key),
            Some(&1)
        );
        assert_eq!(
            runtime.view_render_node_generations.get(&child_key),
            Some(&1)
        );
        assert_eq!(
            runtime.root_render_node_generations.get(&root_key),
            Some(&1)
        );
        assert_eq!(
            runtime
                .render_node_draw_ops
                .get(&map_identity_key(&root_view_render_node)),
            Some(&vec![CanvasDrawOp::RenderNode {
                render_node: child_render_node
            }])
        );
        assert_eq!(
            runtime
                .render_node_draw_ops
                .get(&map_identity_key(&root_target_render_node)),
            Some(&vec![CanvasDrawOp::RenderNode {
                render_node: root_view_render_node.clone()
            }])
        );
        assert_eq!(
            runtime
                .canvas_draw_ops
                .get(&runtime.runtime_canvas_frames[0].canvas_key),
            Some(&vec![CanvasDrawOp::RenderNode {
                render_node: root_target_render_node.clone()
            }])
        );
    }

    {
        let mut runtime = program.runtime.borrow_mut();
        runtime.begin_driven_frame(0);
        runtime.request_draw_traversal();
    }
    assert_eq!(
        program
            .dispatch_runtime_view_frames(420, 760, 0)
            .expect("AOSP ThreadedRenderer should replay a clean root RenderNode target"),
        1
    );
    let runtime = program.runtime.borrow();
    assert_eq!(
        runtime.view_render_node_generations.get(&root_key),
        Some(&1)
    );
    assert_eq!(
        runtime.view_render_node_generations.get(&child_key),
        Some(&1)
    );
    assert_eq!(
        runtime.root_render_node_generations.get(&root_key),
        Some(&1)
    );
    assert_eq!(
        runtime
            .canvas_draw_ops
            .get(&runtime.runtime_canvas_frames[0].canvas_key),
        Some(&vec![CanvasDrawOp::RenderNode {
            render_node: root_target_render_node
        }])
    );
}

#[test]
fn aosp_threaded_renderer_root_bounds_rerecord_without_child_rerecord() {
    let program = DexProgram::new(Vec::new());
    let root = DexValue::Reference(std::rc::Rc::from(
        "Landroid/view/ViewGroup;:threaded-resize-root",
    ));
    let child = DexValue::Reference(std::rc::Rc::from(
        "Landroid/view/View;:threaded-resize-child",
    ));

    write_test_view_bounds(&program, &root, 0, 0, 100, 100);
    write_test_view_bounds(&program, &child, 10, 20, 80, 90);
    {
        let mut runtime = program.runtime.borrow_mut();
        runtime
            .view_group_children
            .insert(map_identity_key(&root), vec![child.clone()]);
        runtime
            .view_parents
            .insert(map_identity_key(&child), root.clone());
        runtime.runtime_view_roots.push(DexRuntimeViewRoot {
            view_key: map_identity_key(&root),
            class_descriptor: "Landroid/view/ViewGroup;".to_owned(),
        });
        runtime.request_draw_traversal();
    }

    assert_eq!(
        program
            .dispatch_runtime_view_frames(420, 760, 0)
            .expect("AOSP ThreadedRenderer should record the initial root target"),
        1
    );
    {
        let mut runtime = program.runtime.borrow_mut();
        runtime.begin_driven_frame(0);
        runtime.request_draw_traversal();
    }
    assert_eq!(
        program
            .dispatch_runtime_view_frames(640, 800, 0)
            .expect("AOSP ThreadedRenderer should re-record root target size changes"),
        1
    );

    let root_key = map_identity_key(&root);
    let child_key = map_identity_key(&child);
    let root_target_render_node = DexValue::Reference(std::rc::Rc::from(format!(
        "Landroid/graphics/RenderNode;:root:{root_key}"
    )));
    let runtime = program.runtime.borrow();
    assert_eq!(
        runtime.view_render_node_generations.get(&root_key),
        Some(&1)
    );
    assert_eq!(
        runtime.view_render_node_generations.get(&child_key),
        Some(&1)
    );
    assert_eq!(
        runtime.root_render_node_generations.get(&root_key),
        Some(&2)
    );
    assert_eq!(
        program.read_stored_instance_field(
            &root_target_render_node,
            &render_node_position_field("positionRight"),
        ),
        Some(DexValue::Int(640))
    );
    assert_eq!(
        program.read_stored_instance_field(
            &root_target_render_node,
            &render_node_position_field("positionBottom"),
        ),
        Some(DexValue::Int(800))
    );
}

#[test]
fn aosp_view_invalidate_recreates_dirty_child_display_list_not_clean_sibling() {
    let program = DexProgram::new(Vec::new());
    let invalidate = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "invalidate".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let root = DexValue::Reference(std::rc::Rc::from(
        "Landroid/view/ViewGroup;:retained-parent",
    ));
    let first = DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:retained-first"));
    let second = DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:retained-second"));
    let initial_canvas =
        DexValue::Reference(std::rc::Rc::from("Landroid/graphics/Canvas;:initial"));
    let redraw_canvas = DexValue::Reference(std::rc::Rc::from("Landroid/graphics/Canvas;:redraw"));

    write_test_view_bounds(&program, &root, 0, 0, 100, 100);
    write_test_view_bounds(&program, &first, 0, 0, 40, 40);
    write_test_view_bounds(&program, &second, 40, 0, 100, 40);
    {
        let mut runtime = program.runtime.borrow_mut();
        runtime
            .view_group_children
            .insert(map_identity_key(&root), vec![first.clone(), second.clone()]);
        runtime
            .view_parents
            .insert(map_identity_key(&first), root.clone());
        runtime
            .view_parents
            .insert(map_identity_key(&second), root.clone());
    }

    program
        .dispatch_runtime_view_draw(&root, &initial_canvas, 0)
        .expect("AOSP View.updateDisplayListIfDirty initial tree draw should record all nodes");
    let root_key = map_identity_key(&root);
    let first_key = map_identity_key(&first);
    let second_key = map_identity_key(&second);

    assert_eq!(
        program.invoke_builtin_method(&invalidate, Some(&test_invocation(vec![first.clone()]))),
        Some(DexExecutionResult::Void)
    );
    program
        .dispatch_runtime_view_draw(&root, &redraw_canvas, 0)
        .expect("AOSP View.invalidate should recreate dirty display lists");

    let runtime = program.runtime.borrow();
    assert_eq!(
        runtime.view_render_node_generations.get(&root_key),
        Some(&2)
    );
    assert_eq!(
        runtime.view_render_node_generations.get(&first_key),
        Some(&2)
    );
    assert_eq!(
        runtime.view_render_node_generations.get(&second_key),
        Some(&1)
    );
    assert_eq!(
        program.read_stored_instance_field(&first, &view_invalidated_field()),
        Some(DexValue::Int(0))
    );
}

#[test]
fn aosp_phonewindow_set_content_view_discards_replaced_content_render_nodes() {
    let program = DexProgram::new(Vec::new());
    let set_content_view = DexMethodId {
        class_descriptor: "Landroid/app/Activity;".to_owned(),
        name: "setContentView".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/view/View;".to_owned()],
        },
    };
    let activity = DexValue::Reference(std::rc::Rc::from("Landroid/app/Activity;:retained"));
    let first = DexValue::Reference(std::rc::Rc::from("Landroid/view/ViewGroup;:content-first"));
    let nested = DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:content-nested"));
    let second = DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:content-second"));
    let content = view_reference(ANDROID_CONTENT_VIEW_ID);

    write_test_view_bounds(&program, &first, 0, 0, 100, 100);
    write_test_view_bounds(&program, &nested, 8, 12, 72, 80);
    write_test_view_bounds(&program, &second, 0, 0, 100, 100);
    {
        let mut runtime = program.runtime.borrow_mut();
        runtime
            .view_group_children
            .insert(map_identity_key(&first), vec![nested.clone()]);
        runtime
            .view_parents
            .insert(map_identity_key(&nested), first.clone());
    }

    assert_eq!(
        program.invoke_builtin_method(
            &set_content_view,
            Some(&test_invocation(vec![activity.clone(), first.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program
            .dispatch_runtime_view_frames(320, 480, 0)
            .expect("AOSP PhoneWindow.setContentView should commit the first content root"),
        1
    );

    let content_key = map_identity_key(&content);
    let first_key = map_identity_key(&first);
    let nested_key = map_identity_key(&nested);
    let second_key = map_identity_key(&second);
    let first_view_render_node = DexValue::Reference(std::rc::Rc::from(format!(
        "Landroid/graphics/RenderNode;:view:{first_key}"
    )));
    let nested_view_render_node = DexValue::Reference(std::rc::Rc::from(format!(
        "Landroid/graphics/RenderNode;:view:{nested_key}"
    )));
    let first_root_render_node = DexValue::Reference(std::rc::Rc::from(format!(
        "Landroid/graphics/RenderNode;:root:{first_key}"
    )));
    {
        let runtime = program.runtime.borrow();
        assert!(runtime.view_render_nodes.contains_key(&first_key));
        assert!(runtime.view_render_nodes.contains_key(&nested_key));
        assert!(runtime.root_render_nodes.contains_key(&first_key));
        assert!(runtime
            .render_node_draw_ops
            .contains_key(&map_identity_key(&first_view_render_node)));
        assert!(runtime
            .render_node_draw_ops
            .contains_key(&map_identity_key(&nested_view_render_node)));
        assert!(runtime
            .render_node_draw_ops
            .contains_key(&map_identity_key(&first_root_render_node)));
    }

    assert_eq!(
        program.invoke_builtin_method(
            &set_content_view,
            Some(&test_invocation(vec![activity, second.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );

    let runtime = program.runtime.borrow();
    assert_eq!(
        runtime.view_group_children.get(&content_key).cloned(),
        Some(vec![second.clone()])
    );
    assert_eq!(runtime.view_parents.get(&first_key), None);
    assert_eq!(runtime.view_parents.get(&second_key), Some(&content));
    assert!(!runtime.view_render_nodes.contains_key(&first_key));
    assert!(!runtime.view_render_nodes.contains_key(&nested_key));
    assert!(!runtime.root_render_nodes.contains_key(&first_key));
    assert!(!runtime
        .render_node_draw_ops
        .contains_key(&map_identity_key(&first_view_render_node)));
    assert!(!runtime
        .render_node_draw_ops
        .contains_key(&map_identity_key(&nested_view_render_node)));
    assert!(!runtime
        .render_node_draw_ops
        .contains_key(&map_identity_key(&first_root_render_node)));
    assert!(runtime.view_render_node_dirty.contains(&content_key));
    assert!(runtime.has_traversal_request());
    assert!(runtime
        .runtime_view_roots
        .iter()
        .any(|root| root.view_key == second_key));
    assert!(!runtime
        .runtime_view_roots
        .iter()
        .any(|root| root.view_key == first_key));
}

#[test]
fn builtin_view_set_system_gesture_exclusion_rects_stores_list() {
    let program = DexProgram::new(Vec::new());
    let set_system_gesture_exclusion_rects = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "setSystemGestureExclusionRects".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/util/List;".to_owned()],
        },
    };
    let view = DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:gestures"));
    let rects = DexValue::Reference(std::rc::Rc::from("Ljava/util/List;:rects"));

    assert_eq!(
        program.invoke_builtin_method(
            &set_system_gesture_exclusion_rects,
            Some(&test_invocation(vec![view.clone(), rects.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&view, &view_system_gesture_exclusion_rects_field()),
        Some(rects)
    );
}

#[test]
fn builtin_view_padding_getters_return_zero() {
    let program = DexProgram::new(Vec::new());

    for name in [
        "getPaddingLeft",
        "getPaddingTop",
        "getPaddingRight",
        "getPaddingBottom",
        "getWidth",
        "getHeight",
    ] {
        let method = DexMethodId {
            class_descriptor: "Landroid/view/View;".to_owned(),
            name: name.to_owned(),
            proto: crate::dex::DexProto {
                shorty: "I".to_owned(),
                return_type: "I".to_owned(),
                parameters: Vec::new(),
            },
        };

        assert_eq!(
            program.invoke_builtin_method(&method, None),
            Some(DexExecutionResult::Int(0))
        );
    }
}

#[test]
fn builtin_view_set_padding_stores_all_four_edges() {
    // cite: frameworks-base View.java#setPadding
    let program = DexProgram::new(Vec::new());
    let set_padding = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "setPadding".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VIIII".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "I".to_owned(),
                "I".to_owned(),
                "I".to_owned(),
                "I".to_owned(),
            ],
        },
    };
    let view = DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:padded"));

    assert_eq!(
        program.invoke_builtin_method(
            &set_padding,
            Some(&test_invocation(vec![
                view.clone(),
                DexValue::Int(1),
                DexValue::Int(2),
                DexValue::Int(3),
                DexValue::Int(4),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );

    let getters = [
        ("getPaddingLeft", 1),
        ("getPaddingTop", 2),
        ("getPaddingRight", 3),
        ("getPaddingBottom", 4),
    ];
    for (name, expected) in getters {
        let getter = DexMethodId {
            class_descriptor: "Landroid/view/View;".to_owned(),
            name: name.to_owned(),
            proto: crate::dex::DexProto {
                shorty: "I".to_owned(),
                return_type: "I".to_owned(),
                parameters: Vec::new(),
            },
        };
        assert_eq!(
            program.invoke_builtin_method(&getter, Some(&test_invocation(vec![view.clone()]))),
            Some(DexExecutionResult::Int(expected))
        );
    }
}

#[test]
fn builtin_view_set_requested_frame_rate_stores_float_state() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "setRequestedFrameRate".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VF".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["F".to_owned()],
        },
    };
    let set_alpha = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "setAlpha".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VF".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["F".to_owned()],
        },
    };
    let get_alpha = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "getAlpha".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "F".to_owned(),
            return_type: "F".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set_elevation = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "setElevation".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VF".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["F".to_owned()],
        },
    };
    let get_elevation = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "getElevation".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "F".to_owned(),
            return_type: "F".to_owned(),
            parameters: Vec::new(),
        },
    };
    let view = DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:frame-rate"));

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![view.clone(), DexValue::Float(60.0)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&view, &view_float_field("requestedFrameRate")),
        Some(DexValue::Float(60.0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                view.clone(),
                DexValue::Int(30.0f32.to_bits() as i32)
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&view, &view_float_field("requestedFrameRate")),
        Some(DexValue::Float(30.0))
    );
    assert_eq!(
        program.invoke_builtin_method(&get_alpha, Some(&test_invocation(vec![view.clone()]))),
        Some(DexExecutionResult::Value(DexValue::Float(1.0)))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_alpha,
            Some(&test_invocation(vec![view.clone(), DexValue::Float(0.5)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&get_alpha, Some(&test_invocation(vec![view.clone()]))),
        Some(DexExecutionResult::Value(DexValue::Float(0.5)))
    );
    assert_eq!(
        program.read_stored_instance_field(&view, &view_float_field("alpha")),
        Some(DexValue::Float(0.5))
    );
    assert_eq!(
        program.invoke_builtin_method(&get_elevation, Some(&test_invocation(vec![view.clone()]))),
        Some(DexExecutionResult::Value(DexValue::Float(0.0)))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_elevation,
            Some(&test_invocation(vec![view.clone(), DexValue::Float(12.0)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&get_elevation, Some(&test_invocation(vec![view.clone()]))),
        Some(DexExecutionResult::Value(DexValue::Float(12.0)))
    );
    assert_eq!(
        program.read_stored_instance_field(&view, &view_float_field("elevation")),
        Some(DexValue::Float(12.0))
    );
}

#[test]
fn builtin_view_get_drawing_time_returns_stored_long_state() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "getDrawingTime".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "J".to_owned(),
            return_type: "J".to_owned(),
            parameters: Vec::new(),
        },
    };
    let view = DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:drawing-time"));

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![view.clone()]))),
        Some(DexExecutionResult::Value(DexValue::Long(0)))
    );
    assert!(program.write_stored_instance_field(
        &view,
        &view_long_field("drawingTime"),
        DexValue::Long(123)
    ));
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![view.clone()]))),
        Some(DexExecutionResult::Value(DexValue::Long(123)))
    );
}

#[test]
fn builtin_view_listener_registration_returns_void() {
    let program = DexProgram::new(Vec::new());
    let insets_listener = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "setOnApplyWindowInsetsListener".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/view/View$OnApplyWindowInsetsListener;".to_owned()],
        },
    };
    let insets_animation_callback_constructor = DexMethodId {
        class_descriptor: "Landroid/view/WindowInsetsAnimation$Callback;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let get_insets_animation_dispatch_mode = DexMethodId {
        class_descriptor: "Landroid/view/WindowInsetsAnimation$Callback;".to_owned(),
        name: "getDispatchMode".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set_insets_animation_callback = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "setWindowInsetsAnimationCallback".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/view/WindowInsetsAnimation$Callback;".to_owned()],
        },
    };
    let set_translation_callback = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "setViewTranslationCallback".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/view/translation/ViewTranslationCallback;".to_owned()],
        },
    };
    let click_listener = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "setOnClickListener".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/view/View$OnClickListener;".to_owned()],
        },
    };
    let long_click_listener = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "setOnLongClickListener".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/view/View$OnLongClickListener;".to_owned()],
        },
    };
    let key_listener = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "setOnKeyListener".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/view/View$OnKeyListener;".to_owned()],
        },
    };
    let touch_listener = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "setOnTouchListener".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/view/View$OnTouchListener;".to_owned()],
        },
    };
    let drag_listener = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "setOnDragListener".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/view/View$OnDragListener;".to_owned()],
        },
    };
    let play_sound_effect = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "playSoundEffect".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let accessibility_delegate_constructor = DexMethodId {
        class_descriptor: "Landroid/view/View$AccessibilityDelegate;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let accessibility_node_provider_constructor = DexMethodId {
        class_descriptor: "Landroid/view/accessibility/AccessibilityNodeProvider;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set_accessibility_delegate = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "setAccessibilityDelegate".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/view/View$AccessibilityDelegate;".to_owned()],
        },
    };
    let post_on_animation = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "postOnAnimation".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/lang/Runnable;".to_owned()],
        },
    };
    let post = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "post".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/Runnable;".to_owned()],
        },
    };
    let post_delayed = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "postDelayed".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZLJ".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/Runnable;".to_owned(), "J".to_owned()],
        },
    };
    let remove_callbacks = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "removeCallbacks".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/Runnable;".to_owned()],
        },
    };
    let post_view = DexValue::Reference(std::rc::Rc::from(
        "Lcom/android/internal/policy/DecorView;.decor",
    ));
    let post_runnable = DexValue::Reference(std::rc::Rc::from("Ljava/lang/Runnable;:frame"));
    let plain_post_runnable = DexValue::Reference(std::rc::Rc::from("Ljava/lang/Runnable;:plain"));
    let delayed_runnable = DexValue::Reference(std::rc::Rc::from("Ljava/lang/Runnable;:delayed"));
    let view_handler = DexValue::Reference(std::rc::Rc::from(format!(
        "Landroid/os/Handler;:view:{}",
        map_identity_key(&post_view)
    )));
    let view_handler_key = map_identity_key(&view_handler);
    let post_invocation = test_invocation(vec![post_view.clone(), post_runnable.clone()]);
    let plain_post_invocation =
        test_invocation(vec![post_view.clone(), plain_post_runnable.clone()]);
    let post_delayed_invocation = test_invocation(vec![
        post_view.clone(),
        delayed_runnable.clone(),
        DexValue::Long(16),
        DexValue::Long(16),
    ]);
    let view = DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:id/2131296342"));
    let listener = DexValue::Object(DexObjectRef {
        id: 7,
        class_descriptor: std::rc::Rc::from("Lexample/ClickListener;"),
    });
    let key_view = DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:id/2131296591"));
    let key_listener_value =
        DexValue::Reference(std::rc::Rc::from("Landroid/view/View$OnKeyListener;:popup"));
    let touch_listener_value = DexValue::Reference(std::rc::Rc::from(
        "Landroid/view/View$OnTouchListener;:popup",
    ));
    let drag_listener_value = DexValue::Reference(std::rc::Rc::from(
        "Landroid/view/View$OnDragListener;:compose",
    ));
    let insets_animation_callback = DexValue::Reference(std::rc::Rc::from(
        "Landroid/view/WindowInsetsAnimation$Callback;:compose",
    ));
    let translation_callback = DexValue::Reference(std::rc::Rc::from(
        "Landroid/view/translation/ViewTranslationCallback;:compose",
    ));
    let click_invocation = test_invocation(vec![view.clone(), listener.clone()]);
    let clear_click_invocation = test_invocation(vec![view, DexValue::Null]);

    assert_eq!(
        program.invoke_builtin_method(&insets_listener, None),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &insets_animation_callback_constructor,
            Some(&test_invocation(vec![
                insets_animation_callback.clone(),
                DexValue::Int(1)
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_insets_animation_dispatch_mode,
            Some(&test_invocation(vec![insets_animation_callback.clone()]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_insets_animation_callback,
            Some(&test_invocation(vec![
                key_view.clone(),
                insets_animation_callback.clone()
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program
            .read_stored_instance_field(&key_view, &view_window_insets_animation_callback_field()),
        Some(insets_animation_callback)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_translation_callback,
            Some(&test_invocation(vec![
                key_view.clone(),
                translation_callback.clone()
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&key_view, &view_translation_callback_field()),
        Some(translation_callback)
    );
    assert_eq!(
        program.invoke_builtin_method(&click_listener, Some(&click_invocation)),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.view_snapshot().click_listeners,
        vec![DexViewClickListener {
            view_id: 2131296342,
            listener: listener.to_string(),
        }]
    );
    assert_eq!(
        program.invoke_builtin_method(&click_listener, Some(&clear_click_invocation)),
        Some(DexExecutionResult::Void)
    );
    assert!(program.view_snapshot().click_listeners.is_empty());
    assert_eq!(
        program.invoke_builtin_method(&long_click_listener, None),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &key_listener,
            Some(&test_invocation(vec![
                key_view.clone(),
                key_listener_value.clone()
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&key_view, &view_key_listener_field()),
        Some(key_listener_value)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &touch_listener,
            Some(&test_invocation(vec![
                key_view.clone(),
                touch_listener_value.clone()
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&key_view, &view_touch_listener_field()),
        Some(touch_listener_value)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &drag_listener,
            Some(&test_invocation(vec![
                key_view.clone(),
                drag_listener_value.clone()
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&key_view, &view_drag_listener_field()),
        Some(drag_listener_value)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &play_sound_effect,
            Some(&test_invocation(vec![key_view.clone(), DexValue::Int(0)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&accessibility_delegate_constructor, None),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&accessibility_node_provider_constructor, None),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&set_accessibility_delegate, None),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&post_on_animation, Some(&post_invocation)),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&post, Some(&plain_post_invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&post_delayed, Some(&post_delayed_invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.runtime.borrow().handler_posts,
        vec![
            HandlerPost {
                handler_key: view_handler_key.clone(),
                handler: Some(view_handler.clone()),
                runnable: post_runnable.clone(),
                delay_millis: 0,
                due_time_millis: 0,
                sequence: 1,
                at_front: false,
            },
            HandlerPost {
                handler_key: view_handler_key.clone(),
                handler: Some(view_handler.clone()),
                runnable: plain_post_runnable.clone(),
                delay_millis: 0,
                due_time_millis: 0,
                sequence: 2,
                at_front: false,
            },
            HandlerPost {
                handler_key: view_handler_key.clone(),
                handler: Some(view_handler.clone()),
                runnable: delayed_runnable.clone(),
                delay_millis: 16,
                due_time_millis: 16,
                sequence: 3,
                at_front: false,
            }
        ]
    );
    assert_eq!(
        program.invoke_builtin_method(&remove_callbacks, Some(&post_invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.runtime.borrow().handler_posts,
        vec![
            HandlerPost {
                handler_key: view_handler_key.clone(),
                handler: Some(view_handler.clone()),
                runnable: plain_post_runnable,
                delay_millis: 0,
                due_time_millis: 0,
                sequence: 2,
                at_front: false,
            },
            HandlerPost {
                handler_key: view_handler_key,
                handler: Some(view_handler.clone()),
                runnable: delayed_runnable,
                delay_millis: 16,
                due_time_millis: 16,
                sequence: 3,
                at_front: false,
            }
        ]
    );
}

#[test]
fn builtin_view_unschedule_drawable_accepts_drawable_or_null() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "unscheduleDrawable".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/graphics/drawable/Drawable;".to_owned()],
        },
    };
    let view = DexValue::Object(DexObjectRef {
        id: 31,
        class_descriptor: std::rc::Rc::from("Landroid/view/View;"),
    });
    let drawable = DexValue::Object(DexObjectRef {
        id: 32,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/drawable/ColorDrawable;"),
    });

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![view.clone(), drawable]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![view, DexValue::Null]))),
        Some(DexExecutionResult::Void)
    );
}

#[test]
fn motion_event_accessors_read_stored_action_and_coordinates() {
    let program = DexProgram::new(Vec::new());
    let action = DexMethodId {
        class_descriptor: "Landroid/view/MotionEvent;".to_owned(),
        name: "getActionMasked".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let meta_state = DexMethodId {
        class_descriptor: "Landroid/view/MotionEvent;".to_owned(),
        name: "getMetaState".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let down_time = DexMethodId {
        class_descriptor: "Landroid/view/MotionEvent;".to_owned(),
        name: "getDownTime".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "J".to_owned(),
            return_type: "J".to_owned(),
            parameters: Vec::new(),
        },
    };
    let event_time = DexMethodId {
        class_descriptor: "Landroid/view/MotionEvent;".to_owned(),
        name: "getEventTime".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "J".to_owned(),
            return_type: "J".to_owned(),
            parameters: Vec::new(),
        },
    };
    let x = DexMethodId {
        class_descriptor: "Landroid/view/MotionEvent;".to_owned(),
        name: "getX".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "F".to_owned(),
            return_type: "F".to_owned(),
            parameters: Vec::new(),
        },
    };
    let indexed_x = DexMethodId {
        class_descriptor: "Landroid/view/MotionEvent;".to_owned(),
        name: "getX".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "FI".to_owned(),
            return_type: "F".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let y = DexMethodId {
        class_descriptor: "Landroid/view/MotionEvent;".to_owned(),
        name: "getY".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "F".to_owned(),
            return_type: "F".to_owned(),
            parameters: Vec::new(),
        },
    };
    let indexed_y = DexMethodId {
        class_descriptor: "Landroid/view/MotionEvent;".to_owned(),
        name: "getY".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "FI".to_owned(),
            return_type: "F".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let pressure = DexMethodId {
        class_descriptor: "Landroid/view/MotionEvent;".to_owned(),
        name: "getPressure".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "FI".to_owned(),
            return_type: "F".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let tool_type = DexMethodId {
        class_descriptor: "Landroid/view/MotionEvent;".to_owned(),
        name: "getToolType".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "II".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let button_state = DexMethodId {
        class_descriptor: "Landroid/view/MotionEvent;".to_owned(),
        name: "getButtonState".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let history_size = DexMethodId {
        class_descriptor: "Landroid/view/MotionEvent;".to_owned(),
        name: "getHistorySize".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let source = DexMethodId {
        class_descriptor: "Landroid/view/MotionEvent;".to_owned(),
        name: "getSource".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let is_from_source = DexMethodId {
        class_descriptor: "Landroid/view/InputEvent;".to_owned(),
        name: "isFromSource".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZI".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let classification = DexMethodId {
        class_descriptor: "Landroid/view/MotionEvent;".to_owned(),
        name: "getClassification".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let obtain_no_history = DexMethodId {
        class_descriptor: "Landroid/view/MotionEvent;".to_owned(),
        name: "obtainNoHistory".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Landroid/view/MotionEvent;".to_owned(),
            parameters: vec!["Landroid/view/MotionEvent;".to_owned()],
        },
    };
    let recycle = DexMethodId {
        class_descriptor: "Landroid/view/MotionEvent;".to_owned(),
        name: "recycle".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let event = program
        .allocate_motion_event_with_times(1, 32.5, 48.25, 123, 123)
        .expect("MotionEvent should allocate");
    let invocation = test_invocation(vec![event.clone()]);

    assert_eq!(
        program.invoke_builtin_method(&action, Some(&invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&meta_state, Some(&invocation)),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&down_time, Some(&invocation)),
        Some(DexExecutionResult::Value(DexValue::Long(123)))
    );
    assert_eq!(
        program.invoke_builtin_method(&event_time, Some(&invocation)),
        Some(DexExecutionResult::Value(DexValue::Long(123)))
    );
    assert_eq!(
        program.invoke_builtin_method(&x, Some(&invocation)),
        Some(DexExecutionResult::Value(DexValue::Float(32.5)))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &indexed_x,
            Some(&test_invocation(vec![event.clone(), DexValue::Int(0)]))
        ),
        Some(DexExecutionResult::Value(DexValue::Float(32.5)))
    );
    assert_eq!(
        program.invoke_builtin_method(&y, Some(&invocation)),
        Some(DexExecutionResult::Value(DexValue::Float(48.25)))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &indexed_y,
            Some(&test_invocation(vec![event.clone(), DexValue::Int(0)]))
        ),
        Some(DexExecutionResult::Value(DexValue::Float(48.25)))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &pressure,
            Some(&test_invocation(vec![event.clone(), DexValue::Int(0)]))
        ),
        Some(DexExecutionResult::Value(DexValue::Float(1.0)))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &tool_type,
            Some(&test_invocation(vec![event.clone(), DexValue::Int(0)]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&button_state, Some(&invocation)),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&history_size, Some(&invocation)),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&source, Some(&invocation)),
        Some(DexExecutionResult::Int(0x0000_1002))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &is_from_source,
            Some(&test_invocation(vec![
                event.clone(),
                DexValue::Int(0x0000_1002)
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &is_from_source,
            Some(&test_invocation(vec![
                event.clone(),
                DexValue::Int(0x0000_2002)
            ]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&classification, Some(&invocation)),
        Some(DexExecutionResult::Int(0))
    );
    program.runtime.borrow_mut().main_thread_uptime_millis = 456;
    let Some(DexExecutionResult::Value(copied)) =
        program.invoke_builtin_method(&obtain_no_history, Some(&test_invocation(vec![event])))
    else {
        panic!("MotionEvent.obtainNoHistory should return a copied event");
    };
    let copied_invocation = test_invocation(vec![copied]);
    assert_eq!(
        program.invoke_builtin_method(&action, Some(&copied_invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&x, Some(&copied_invocation)),
        Some(DexExecutionResult::Value(DexValue::Float(32.5)))
    );
    assert_eq!(
        program.invoke_builtin_method(&down_time, Some(&copied_invocation)),
        Some(DexExecutionResult::Value(DexValue::Long(123)))
    );
    assert_eq!(
        program.invoke_builtin_method(&event_time, Some(&copied_invocation)),
        Some(DexExecutionResult::Value(DexValue::Long(123)))
    );
    assert_eq!(
        program.invoke_builtin_method(&recycle, Some(&copied_invocation)),
        Some(DexExecutionResult::Void)
    );
}

#[test]
fn motion_event_tap_up_keeps_original_down_time() {
    let program = DexProgram::new(Vec::new());
    let down_time = DexMethodId {
        class_descriptor: "Landroid/view/MotionEvent;".to_owned(),
        name: "getDownTime".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "J".to_owned(),
            return_type: "J".to_owned(),
            parameters: Vec::new(),
        },
    };
    let event_time = DexMethodId {
        class_descriptor: "Landroid/view/MotionEvent;".to_owned(),
        name: "getEventTime".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "J".to_owned(),
            return_type: "J".to_owned(),
            parameters: Vec::new(),
        },
    };
    let action = DexMethodId {
        class_descriptor: "Landroid/view/MotionEvent;".to_owned(),
        name: "getActionMasked".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let up_event = program
        .allocate_motion_event_with_times(1, 10.0, 12.0, 200, 216)
        .expect("MotionEvent should allocate");
    let invocation = test_invocation(vec![up_event]);

    assert_eq!(
        program.invoke_builtin_method(&action, Some(&invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&down_time, Some(&invocation)),
        Some(DexExecutionResult::Value(DexValue::Long(200)))
    );
    assert_eq!(
        program.invoke_builtin_method(&event_time, Some(&invocation)),
        Some(DexExecutionResult::Value(DexValue::Long(216)))
    );
}

#[test]
fn view_perform_click_without_listener_returns_false() {
    let program = DexProgram::new(Vec::new());
    let invocation = test_invocation(vec![DexValue::Reference(std::rc::Rc::from(
        "Landroid/view/View;:id/2131296342",
    ))]);

    assert_eq!(program.perform_view_click(&invocation, 0).unwrap(), 0);
}

#[test]
fn activity_runtime_click_view_id_dispatches_by_android_id() {
    let runtime = DexActivityRuntime {
        program: DexProgram::new(Vec::new()),
        class_descriptor: "Lexample/MainActivity;".to_owned(),
        method_signature: "onCreate(Landroid/os/Bundle;)V".to_owned(),
        result: "void".to_owned(),
        last_view_snapshot: std::cell::RefCell::new(None),
    };

    assert_eq!(runtime.click_view_id(2131296342).unwrap(), 0);
}

#[test]
// cite: androidx ListMenuPresenter.java#onItemClick
// cite: androidx MenuBuilder.java#performItemAction
// cite: androidx MenuItemImpl.java#invoke
// cite: androidx AppCompatDelegateImpl.java#onMenuItemSelected
fn androidx_menuitemimpl_invoke_routes_options_item_before_popup_overlay_dismissal() {
    let runtime = DexActivityRuntime {
        program: DexProgram::new(Vec::new()),
        class_descriptor: "Lexample/MainActivity;".to_owned(),
        method_signature: "onCreate(Landroid/os/Bundle;)V".to_owned(),
        result: "void".to_owned(),
        last_view_snapshot: std::cell::RefCell::new(None),
    };
    let item_id = 2131296357;

    {
        let mut state = runtime.program.runtime.borrow_mut();
        state.action_bar_overflow_open = true;
        state.options_menu.push(RuntimeMenuItem {
            item_id,
            title: "Settings".to_owned(),
            enabled: true,
            visible: true,
            on_click: None,
            icon_resource: None,
            show_as_action: 0,
        });
        state.active_menu_overlays.push(RuntimeMenuOverlay {
            popup_key: "overflow-popup".to_owned(),
            anchor_view_id: Some(2131296591),
            list_view: DexValue::Reference(std::rc::Rc::from("Landroid/widget/ListView;:overflow")),
            items: vec![RuntimeMenuOverlayItem {
                item_id,
                title: "Settings".to_owned(),
                enabled: true,
                position: 0,
                on_click: None,
            }],
            menu_item_click_listener: None,
        });
    }

    assert_eq!(runtime.click_view_id(item_id).unwrap(), 0);

    let state = runtime.program.runtime.borrow();
    assert!(!state.action_bar_overflow_open);
    assert!(state.active_menu_overlays.is_empty());
}

#[test]
// cite: frameworks-base PopupWindow.java#dismiss
fn popupwindow_dismiss_visible_overlay_changes_view_snapshot() {
    let program = DexProgram::new(Vec::new());
    let empty = program.view_snapshot();

    program
        .runtime
        .borrow_mut()
        .active_menu_overlays
        .push(RuntimeMenuOverlay {
            popup_key: "overflow-popup".to_owned(),
            anchor_view_id: Some(2131296591),
            list_view: DexValue::Reference(std::rc::Rc::from("Landroid/widget/ListView;:overflow")),
            items: vec![RuntimeMenuOverlayItem {
                item_id: 2131296357,
                title: "Settings".to_owned(),
                enabled: true,
                position: 0,
                on_click: None,
            }],
            menu_item_click_listener: None,
        });

    let with_overlay = program.view_snapshot();
    program.runtime.borrow_mut().active_menu_overlays.clear();
    let dismissed = program.view_snapshot();

    assert_ne!(empty, with_overlay);
    assert_eq!(empty, dismissed);
}

#[test]
fn activity_runtime_click_view_id_dispatches_dialog_scoped_child_listener() {
    let runtime = DexActivityRuntime {
        program: DexProgram::new(Vec::new()),
        class_descriptor: "Lexample/MainActivity;".to_owned(),
        method_signature: "onCreate(Landroid/os/Bundle;)V".to_owned(),
        result: "void".to_owned(),
        last_view_snapshot: std::cell::RefCell::new(None),
    };
    let layout_id = 2131492910;
    let view_id = 2131296680;
    let root = layout_reference(layout_id);
    let child = layout_child_reference(layout_id, view_id, "Landroid/widget/ImageView;");
    let listener = DexValue::Object(DexObjectRef {
        id: 7,
        class_descriptor: std::rc::Rc::from("Lexample/ClickListener;"),
    });

    runtime
        .program
        .write_stored_instance_field(&child, &view_id_field(), DexValue::Int(view_id));
    {
        let mut state = runtime.program.runtime.borrow_mut();
        state.active_dialog_overlays.push(RuntimeDialogOverlay {
            dialog_key: "support".to_owned(),
            title: Some("Support Us".to_owned()),
            content_layouts: vec![layout_id],
            items: Vec::new(),
            buttons: Vec::new(),
            input_value: None,
        });
        state
            .inflated_view_children
            .insert((map_identity_key(&root), view_id), child.clone());
        state
            .view_click_listeners
            .insert(map_identity_key(&child), listener);
    }

    let error = runtime
        .click_view_id(view_id)
        .expect_err("missing listener onClick should be reached through scoped child");

    assert!(
        error.to_string().contains("OnClickListener.onClick"),
        "unexpected error: {error}"
    );
}

#[test]
fn activity_runtime_click_view_id_falls_back_to_synthetic_xml_onclick() {
    let runtime = DexActivityRuntime {
        program: DexProgram::new(Vec::new()),
        class_descriptor: "Lexample/MainActivity;".to_owned(),
        method_signature: "onCreate(Landroid/os/Bundle;)V".to_owned(),
        result: "void".to_owned(),
        last_view_snapshot: std::cell::RefCell::new(None),
    };
    let layout_id = 2131492910;
    let view_id = 2131296342;
    let root = layout_reference(layout_id);
    let child = layout_child_reference(layout_id, view_id, "Landroid/widget/Button;");
    let activity = DexValue::Object(DexObjectRef {
        id: 1,
        class_descriptor: std::rc::Rc::from("Lexample/MainActivity;"),
    });

    runtime.program.write_activity_receiver(activity);
    runtime
        .register_declared_on_click_handler(view_id, "addButton")
        .expect("declared onClick should register on the synthetic view id");
    {
        let mut state = runtime.program.runtime.borrow_mut();
        state.active_dialog_overlays.push(RuntimeDialogOverlay {
            dialog_key: "dialog".to_owned(),
            title: None,
            content_layouts: vec![layout_id],
            items: Vec::new(),
            buttons: Vec::new(),
            input_value: None,
        });
        state
            .inflated_view_children
            .insert((map_identity_key(&root), view_id), child);
    }

    let error = runtime
        .click_view_id(view_id)
        .expect_err("synthetic XML onClick fallback should still run");

    assert!(
        error
            .to_string()
            .contains("XML onClick handler method was not found"),
        "unexpected error: {error}"
    );
}

#[test]
fn activity_stack_scopes_snapshot_to_top_activity() {
    let program = DexProgram::new(Vec::new());
    let main_activity = DexValue::Object(DexObjectRef {
        id: 1,
        class_descriptor: std::rc::Rc::from("Lexample/MainActivity;"),
    });
    let settings_activity = DexValue::Object(DexObjectRef {
        id: 2,
        class_descriptor: std::rc::Rc::from("Lexample/SettingsActivity;"),
    });

    program.push_activity(
        "Lexample/MainActivity;".to_owned(),
        main_activity,
        "onCreate(Landroid/os/Bundle;)V".to_owned(),
    );
    {
        let mut runtime = program.runtime.borrow_mut();
        runtime.content_layouts.push(100);
        runtime.inflated_layouts.push(101);
        runtime.view_lookup_ids.push(102);
    }
    program.push_activity(
        "Lexample/SettingsActivity;".to_owned(),
        settings_activity.clone(),
        "onCreate(Landroid/os/Bundle;)V".to_owned(),
    );
    {
        let mut runtime = program.runtime.borrow_mut();
        runtime.content_layouts.push(200);
        runtime.inflated_layouts.push(201);
        runtime.view_lookup_ids.push(202);
    }
    // cite: frameworks-base ViewGroup.java#addView
    program.append_view_child(
        &view_reference(ANDROID_CONTENT_VIEW_ID),
        &program.layout_reference_for_id(201),
    );

    assert_eq!(program.view_snapshot().content_layouts, vec![200]);
    assert_eq!(program.view_snapshot().inflated_layouts, vec![201]);
    assert_eq!(program.view_snapshot().view_lookup_ids, vec![202]);
    assert_eq!(program.active_activity_receiver(), Some(settings_activity));
}

#[test]
fn activity_finish_pops_top_activity_and_restores_previous_snapshot() {
    let program = DexProgram::new(Vec::new());
    let main_activity = DexValue::Object(DexObjectRef {
        id: 1,
        class_descriptor: std::rc::Rc::from("Lexample/MainActivity;"),
    });
    let about_activity = DexValue::Object(DexObjectRef {
        id: 2,
        class_descriptor: std::rc::Rc::from("Lexample/AboutActivity;"),
    });
    program.push_activity(
        "Lexample/MainActivity;".to_owned(),
        main_activity.clone(),
        "onCreate(Landroid/os/Bundle;)V".to_owned(),
    );
    {
        let mut runtime = program.runtime.borrow_mut();
        runtime.content_layouts.push(100);
        runtime.inflated_layouts.push(101);
        runtime.dialog_content_layouts.push(102);
        runtime.view_lookup_ids.push(103);
    }
    program.push_activity(
        "Lexample/AboutActivity;".to_owned(),
        about_activity.clone(),
        "onCreate(Landroid/os/Bundle;)V".to_owned(),
    );
    {
        let mut runtime = program.runtime.borrow_mut();
        runtime.content_layouts.push(200);
        runtime.inflated_layouts.push(201);
        runtime.dialog_content_layouts.push(202);
        runtime.view_lookup_ids.push(203);
    }

    program
        .finish_activity(&test_invocation(vec![about_activity]), 0)
        .expect("finishing the top Activity should succeed");

    // cite: frameworks-base ViewGroup.java#addView
    program.append_view_child(
        &view_reference(ANDROID_CONTENT_VIEW_ID),
        &program.layout_reference_for_id(101),
    );

    assert_eq!(program.view_snapshot().content_layouts, vec![100]);
    assert_eq!(program.view_snapshot().inflated_layouts, vec![101]);
    assert_eq!(program.view_snapshot().view_lookup_ids, vec![103]);
    assert_eq!(program.dialog_content_layouts(), vec![102]);
    assert_eq!(program.active_activity_receiver(), Some(main_activity));
}

#[test]
fn content_view_recording_reads_layout_id_from_view_reference() {
    let program = DexProgram::new(Vec::new());
    let invocation = test_invocation(vec![
        DexValue::Object(DexObjectRef {
            id: 1,
            class_descriptor: std::rc::Rc::from("Lg/m;"),
        }),
        DexValue::Reference(std::rc::Rc::from(
            "Landroidx/core/widget/NestedScrollView;:layout/2131492892",
        )),
    ]);

    assert!(program.record_content_view_argument(&invocation, 1));
    assert_eq!(program.view_snapshot().content_layouts, vec![2131492892]);
}

#[test]
fn resource_content_view_links_inflated_root_so_later_live_children_are_findable() {
    // cite: frameworks-base PhoneWindow.java#setContentView, generateLayout
    // cite: frameworks-base ViewGroup.java#addView
    let program = DexProgram::new(Vec::new());
    let invocation = test_invocation(vec![
        DexValue::Object(DexObjectRef {
            id: 1,
            class_descriptor: std::rc::Rc::from("Lexample/MainActivity;"),
        }),
        DexValue::Int(2130968576),
    ]);
    assert!(program.record_content_view_argument(&invocation, 1));

    let root = program.layout_reference_for_id(2130968576);
    let live_view = DexValue::Object(DexObjectRef {
        id: 900,
        class_descriptor: std::rc::Rc::from("Landroid/widget/TextView;"),
    });
    let add_view = DexMethodId {
        class_descriptor: "Landroid/view/ViewGroup;".to_owned(),
        name: "addView".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/view/View;".to_owned()],
        },
    };
    let set_id = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "setId".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let find_view_by_id = DexMethodId {
        class_descriptor: "Landroid/app/Activity;".to_owned(),
        name: "findViewById".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LI".to_owned(),
            return_type: "Landroid/view/View;".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    const LIVE_VIEW_ID: i32 = i32::MAX;

    assert_eq!(
        program.invoke_builtin_method(
            &add_view,
            Some(&test_invocation(vec![root, live_view.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_id,
            Some(&test_invocation(vec![
                live_view.clone(),
                DexValue::Int(LIVE_VIEW_ID)
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &find_view_by_id,
            Some(&test_invocation(vec![
                DexValue::Reference(std::rc::Rc::from("Lexample/MainActivity;:activity")),
                DexValue::Int(LIVE_VIEW_ID)
            ]))
        ),
        Some(DexExecutionResult::Value(live_view))
    );
}

#[test]
fn start_activity_from_implicit_intent_errors_at_named_primitive() {
    let program = DexProgram::new(Vec::new());
    let invocation = test_invocation(vec![
        DexValue::Object(DexObjectRef {
            id: 1,
            class_descriptor: std::rc::Rc::from("Lexample/MainActivity;"),
        }),
        DexValue::Object(DexObjectRef {
            id: 2,
            class_descriptor: std::rc::Rc::from("Landroid/content/Intent;"),
        }),
        DexValue::Null,
    ]);
    let error = program
        .start_activity_from_intent(&invocation, 0)
        .expect_err("implicit Intent should fail explicitly");

    assert!(
        error
            .to_string()
            .contains("Android implicit Intent activity resolution"),
        "unexpected error: {error}"
    );
}

#[test]
fn start_activity_from_action_view_web_intent_enqueues_browser_action() {
    let program = DexProgram::new(Vec::new());
    let intent = DexValue::Object(DexObjectRef {
        id: 2,
        class_descriptor: std::rc::Rc::from("Landroid/content/Intent;"),
    });
    let uri = DexValue::Object(DexObjectRef {
        id: 3,
        class_descriptor: std::rc::Rc::from("Landroid/net/Uri;"),
    });
    let url = "https://github.com/Darkempire78/OpenCalc".to_owned();
    program.write_stored_instance_field(
        &intent,
        &intent_action_field(),
        DexValue::String(std::rc::Rc::from(ANDROID_INTENT_ACTION_VIEW.to_owned())),
    );
    program.write_stored_instance_field(&intent, &intent_data_uri_field(), uri.clone());
    program.write_stored_instance_field(
        &uri,
        &uri_string_field(),
        DexValue::String(std::rc::Rc::from(url.clone())),
    );

    let invocation = test_invocation(vec![
        DexValue::Object(DexObjectRef {
            id: 1,
            class_descriptor: std::rc::Rc::from("Lexample/MainActivity;"),
        }),
        intent,
    ]);

    program
        .start_activity_from_intent(&invocation, 0)
        .expect("ACTION_VIEW web Intent should enqueue a browser action");

    assert_eq!(
        program.take_host_actions(),
        vec![DexHostAction::OpenBrowser { url }]
    );
}

#[test]
fn start_activity_from_non_web_action_view_intent_errors_at_named_primitive() {
    let program = DexProgram::new(Vec::new());
    let intent = DexValue::Object(DexObjectRef {
        id: 2,
        class_descriptor: std::rc::Rc::from("Landroid/content/Intent;"),
    });
    let uri = DexValue::Object(DexObjectRef {
        id: 3,
        class_descriptor: std::rc::Rc::from("Landroid/net/Uri;"),
    });
    program.write_stored_instance_field(
        &intent,
        &intent_action_field(),
        DexValue::String(std::rc::Rc::from(ANDROID_INTENT_ACTION_VIEW.to_owned())),
    );
    program.write_stored_instance_field(&intent, &intent_data_uri_field(), uri.clone());
    program.write_stored_instance_field(
        &uri,
        &uri_string_field(),
        DexValue::String(std::rc::Rc::from("mailto:team@example.com")),
    );

    let invocation = test_invocation(vec![
        DexValue::Object(DexObjectRef {
            id: 1,
            class_descriptor: std::rc::Rc::from("Lexample/MainActivity;"),
        }),
        intent,
    ]);
    let error = program
        .start_activity_from_intent(&invocation, 0)
        .expect_err("non-web ACTION_VIEW Intent should fail explicitly");

    assert!(
        error
            .to_string()
            .contains("Android non-browser ACTION_VIEW handling"),
        "unexpected error: {error}"
    );
}

#[test]
fn activity_runtime_registers_declared_on_click_handler() {
    let runtime = DexActivityRuntime {
        program: DexProgram::new(Vec::new()),
        class_descriptor: "Lexample/MainActivity;".to_owned(),
        method_signature: "onCreate(Landroid/os/Bundle;)V".to_owned(),
        result: "void".to_owned(),
        last_view_snapshot: std::cell::RefCell::new(None),
    };

    runtime
        .register_declared_on_click_handler(2131296342, "addButton")
        .expect("declared onClick should register");

    assert_eq!(
        runtime.view_snapshot().declared_on_click_handlers,
        vec![DexDeclaredOnClickHandler {
            view_id: 2131296342,
            method_name: "addButton".to_owned(),
        }]
    );
}

#[test]
fn activity_runtime_declared_on_click_errors_when_activity_method_is_missing() {
    let runtime = DexActivityRuntime {
        program: DexProgram::new(Vec::new()),
        class_descriptor: "Lexample/MainActivity;".to_owned(),
        method_signature: "onCreate(Landroid/os/Bundle;)V".to_owned(),
        result: "void".to_owned(),
        last_view_snapshot: std::cell::RefCell::new(None),
    };
    let activity = DexValue::Object(DexObjectRef {
        id: 1,
        class_descriptor: std::rc::Rc::from("Lexample/MainActivity;"),
    });

    runtime.program.write_activity_receiver(activity);
    runtime
        .register_declared_on_click_handler(2131296342, "addButton")
        .expect("declared onClick should register");
    let error = runtime
        .click_view_id(2131296342)
        .expect_err("missing Activity method should fail explicitly");

    assert!(
        error
            .to_string()
            .contains("XML onClick handler method was not found"),
        "unexpected error: {error}"
    );
    assert!(
        error.to_string().contains("next missing primitive:"),
        "unexpected error: {error}"
    );
}

#[test]
fn view_perform_click_errors_when_listener_onclick_is_missing() {
    let program = DexProgram::new(Vec::new());
    let set_listener = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "setOnClickListener".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/view/View$OnClickListener;".to_owned()],
        },
    };
    let view = DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:id/2131296342"));
    let listener = DexValue::Object(DexObjectRef {
        id: 7,
        class_descriptor: std::rc::Rc::from("Lexample/ClickListener;"),
    });
    let set_invocation = test_invocation(vec![view.clone(), listener]);
    let click_invocation = test_invocation(vec![view]);

    assert_eq!(
        program.invoke_builtin_method(&set_listener, Some(&set_invocation)),
        Some(DexExecutionResult::Void)
    );
    let error = program
        .perform_view_click(&click_invocation, 0)
        .expect_err("missing listener onClick should fail explicitly");

    assert!(
        error.to_string().contains("OnClickListener.onClick"),
        "unexpected error: {error}"
    );
    assert!(
        error.to_string().contains("next missing primitive:"),
        "unexpected error: {error}"
    );
}

#[test]
fn builtin_view_group_descendant_focusability_round_trips() {
    let program = DexProgram::new(Vec::new());
    let set_descendant_focusability = DexMethodId {
        class_descriptor: "Landroid/view/ViewGroup;".to_owned(),
        name: "setDescendantFocusability".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let get_descendant_focusability = DexMethodId {
        class_descriptor: "Landroid/view/ViewGroup;".to_owned(),
        name: "getDescendantFocusability".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let view_group =
        DexValue::Reference(std::rc::Rc::from("Landroid/view/ViewGroup;:nested-scroll"));

    assert_eq!(
        program.invoke_builtin_method(
            &get_descendant_focusability,
            Some(&test_invocation(vec![view_group.clone()]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_descendant_focusability,
            Some(&test_invocation(vec![
                view_group.clone(),
                DexValue::Int(262144)
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_descendant_focusability,
            Some(&test_invocation(vec![view_group.clone()]))
        ),
        Some(DexExecutionResult::Int(262144))
    );
    assert_eq!(
        program
            .read_stored_instance_field(&view_group, &view_group_descendant_focusability_field()),
        Some(DexValue::Int(262144))
    );
}

#[test]
fn builtin_view_group_clip_flags_round_trip() {
    let program = DexProgram::new(Vec::new());
    let view_group =
        DexValue::Reference(std::rc::Rc::from("Landroid/view/ViewGroup;:compose-host"));
    let set_clip_children = DexMethodId {
        class_descriptor: "Landroid/view/ViewGroup;".to_owned(),
        name: "setClipChildren".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VZ".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Z".to_owned()],
        },
    };
    let get_clip_children = DexMethodId {
        class_descriptor: "Landroid/view/ViewGroup;".to_owned(),
        name: "getClipChildren".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set_clip_to_padding = DexMethodId {
        class_descriptor: "Landroid/view/ViewGroup;".to_owned(),
        name: "setClipToPadding".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VZ".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Z".to_owned()],
        },
    };
    let get_clip_to_padding = DexMethodId {
        class_descriptor: "Landroid/view/ViewGroup;".to_owned(),
        name: "getClipToPadding".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };

    assert_eq!(
        program.invoke_builtin_method(
            &get_clip_children,
            Some(&test_invocation(vec![view_group.clone()]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_clip_children,
            Some(&test_invocation(vec![view_group.clone(), DexValue::Int(0)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_clip_children,
            Some(&test_invocation(vec![view_group.clone()]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_clip_to_padding,
            Some(&test_invocation(vec![view_group.clone(), DexValue::Int(0)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_clip_to_padding,
            Some(&test_invocation(vec![view_group]))
        ),
        Some(DexExecutionResult::Int(0))
    );
}

#[test]
fn builtin_choreographer_main_instance_and_callbacks_are_available() {
    let program = DexProgram::new(Vec::new());
    let get_instance = DexMethodId {
        class_descriptor: "Landroid/view/Choreographer;".to_owned(),
        name: "getInstance".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/view/Choreographer;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let post_frame_callback = DexMethodId {
        class_descriptor: "Landroid/view/Choreographer;".to_owned(),
        name: "postFrameCallback".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/view/Choreographer$FrameCallback;".to_owned()],
        },
    };
    let post_frame_callback_delayed = DexMethodId {
        class_descriptor: "Landroid/view/Choreographer;".to_owned(),
        name: "postFrameCallbackDelayed".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VLJ".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "Landroid/view/Choreographer$FrameCallback;".to_owned(),
                "J".to_owned(),
            ],
        },
    };
    let remove_frame_callback = DexMethodId {
        class_descriptor: "Landroid/view/Choreographer;".to_owned(),
        name: "removeFrameCallback".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/view/Choreographer$FrameCallback;".to_owned()],
        },
    };
    let remove_callbacks = DexMethodId {
        class_descriptor: "Landroid/view/Choreographer;".to_owned(),
        name: "removeCallbacks".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VILL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "I".to_owned(),
                "Ljava/lang/Runnable;".to_owned(),
                "Ljava/lang/Object;".to_owned(),
            ],
        },
    };
    let post_callback = DexMethodId {
        class_descriptor: "Landroid/view/Choreographer;".to_owned(),
        name: "postCallback".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VILL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "I".to_owned(),
                "Ljava/lang/Runnable;".to_owned(),
                "Ljava/lang/Object;".to_owned(),
            ],
        },
    };
    let post_callback_delayed = DexMethodId {
        class_descriptor: "Landroid/view/Choreographer;".to_owned(),
        name: "postCallbackDelayed".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VILLJ".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "I".to_owned(),
                "Ljava/lang/Runnable;".to_owned(),
                "Ljava/lang/Object;".to_owned(),
                "J".to_owned(),
            ],
        },
    };
    let choreographer = DexValue::Reference(std::rc::Rc::from("Landroid/view/Choreographer;:main"));
    let callback = DexValue::Reference(std::rc::Rc::from(
        "Landroid/view/Choreographer$FrameCallback;:test",
    ));
    let delayed_callback = DexValue::Reference(std::rc::Rc::from(
        "Landroid/view/Choreographer$FrameCallback;:delayed",
    ));
    let runnable = DexValue::Reference(std::rc::Rc::from("Ljava/lang/Runnable;:frame"));
    let token = DexValue::Reference(std::rc::Rc::from("Ljava/lang/Object;:token"));

    assert_eq!(
        program.invoke_builtin_method(&get_instance, None),
        Some(DexExecutionResult::Value(choreographer.clone()))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &post_frame_callback,
            Some(&test_invocation(vec![
                choreographer.clone(),
                callback.clone()
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.runtime.borrow().choreographer_frame_callbacks,
        vec![callback.clone()]
    );
    assert_eq!(
        program.invoke_builtin_method(
            &remove_frame_callback,
            Some(&test_invocation(vec![choreographer.clone(), callback]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert!(program
        .runtime
        .borrow()
        .choreographer_frame_callbacks
        .is_empty());
    assert_eq!(
        program.invoke_builtin_method(
            &post_frame_callback_delayed,
            Some(&test_invocation(vec![
                choreographer.clone(),
                delayed_callback.clone(),
                DexValue::Long(16),
                DexValue::Long(16),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .choreographer_delayed_frame_callbacks
            .len(),
        1
    );
    {
        let mut runtime = program.runtime.borrow_mut();
        runtime.advance_main_thread_uptime_to_frame(20_000_000);
        let work = runtime.take_main_thread_work_with_limit(true, None);
        assert_eq!(work.frame_callbacks, vec![delayed_callback]);
    }
    assert_eq!(
        program.invoke_builtin_method(
            &post_callback,
            Some(&test_invocation(vec![
                choreographer.clone(),
                DexValue::Int(3),
                runnable.clone(),
                token.clone(),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .choreographer_runnable_callbacks
            .len(),
        1
    );
    assert_eq!(
        program.invoke_builtin_method(
            &remove_callbacks,
            Some(&test_invocation(vec![
                choreographer.clone(),
                DexValue::Int(3),
                runnable.clone(),
                DexValue::Null,
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert!(program
        .runtime
        .borrow()
        .choreographer_runnable_callbacks
        .is_empty());
    assert_eq!(
        program.invoke_builtin_method(
            &post_callback_delayed,
            Some(&test_invocation(vec![
                choreographer.clone(),
                DexValue::Int(1),
                runnable,
                token,
                DexValue::Long(32),
                DexValue::Long(32),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.runtime.borrow().choreographer_runnable_callbacks[0].due_time_millis,
        52
    );
    assert_eq!(
        program.invoke_builtin_method(
            &remove_callbacks,
            Some(&test_invocation(vec![
                choreographer,
                DexValue::Int(0),
                DexValue::Null,
                DexValue::Null,
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
}

#[test]
fn ui_thread_pump_dispatches_choreographer_frame_callbacks() {
    let program = DexProgram::new(Vec::new());
    let callback = DexValue::Object(DexObjectRef {
        id: 7,
        class_descriptor: std::rc::Rc::from("Lexample/FrameCallback;"),
    });
    program
        .runtime
        .borrow_mut()
        .choreographer_frame_callbacks
        .push(callback);

    let error = program
        .dispatch_pending_main_loop(420, 760, 0)
        .expect_err("missing doFrame callback method should fail explicitly");

    assert!(
        error
            .to_string()
            .contains("Choreographer.FrameCallback callback method was not found"),
        "unexpected error: {error}"
    );
    assert!(program
        .runtime
        .borrow()
        .choreographer_frame_callbacks
        .is_empty());
}

#[test]
fn ui_thread_work_can_leave_next_frame_callbacks_queued() {
    let program = DexProgram::new(Vec::new());
    let first_callback = DexValue::Object(DexObjectRef {
        id: 7,
        class_descriptor: std::rc::Rc::from("Lexample/FrameCallback;"),
    });
    let next_callback = DexValue::Object(DexObjectRef {
        id: 8,
        class_descriptor: std::rc::Rc::from("Lexample/FrameCallback;"),
    });

    let mut runtime = program.runtime.borrow_mut();
    runtime
        .choreographer_frame_callbacks
        .push(first_callback.clone());

    let work = runtime.take_main_thread_work_with_limit(true, None);
    assert_eq!(work.frame_callbacks, vec![first_callback]);
    assert!(runtime.choreographer_frame_callbacks.is_empty());

    runtime
        .choreographer_frame_callbacks
        .push(next_callback.clone());
    let work = runtime.take_main_thread_work_with_limit(false, None);

    assert!(work.frame_callbacks.is_empty());
    assert_eq!(runtime.choreographer_frame_callbacks, vec![next_callback]);
}

#[test]
fn ui_thread_work_limit_leaves_extra_frame_callbacks_queued() {
    let program = DexProgram::new(Vec::new());
    let first_callback = DexValue::Object(DexObjectRef {
        id: 7,
        class_descriptor: std::rc::Rc::from("Lexample/FrameCallback;"),
    });
    let second_callback = DexValue::Object(DexObjectRef {
        id: 8,
        class_descriptor: std::rc::Rc::from("Lexample/FrameCallback;"),
    });

    let mut runtime = program.runtime.borrow_mut();
    runtime
        .choreographer_frame_callbacks
        .push(first_callback.clone());
    runtime
        .choreographer_frame_callbacks
        .push(second_callback.clone());

    let work = runtime.take_main_thread_work_with_limit(true, Some(1));

    assert_eq!(work.frame_callbacks, vec![first_callback]);
    assert_eq!(runtime.choreographer_frame_callbacks, vec![second_callback]);
}

#[test]
fn ui_thread_work_limit_leaves_extra_due_handler_posts_queued() {
    let program = DexProgram::new(Vec::new());
    let first_runnable = DexValue::Object(DexObjectRef {
        id: 7,
        class_descriptor: std::rc::Rc::from("Lexample/Runnable;"),
    });
    let second_runnable = DexValue::Object(DexObjectRef {
        id: 8,
        class_descriptor: std::rc::Rc::from("Lexample/Runnable;"),
    });

    let mut runtime = program.runtime.borrow_mut();
    runtime.post_handler_runnable(
        "reference:handler".to_owned(),
        first_runnable.clone(),
        0,
        false,
    );
    runtime.post_handler_runnable(
        "reference:handler".to_owned(),
        second_runnable.clone(),
        0,
        false,
    );

    let work = runtime.take_main_thread_work_with_limit(true, Some(1));

    assert_eq!(work.handler_posts.len(), 1);
    assert_eq!(work.handler_posts[0].runnable, first_runnable);
    assert_eq!(runtime.handler_posts.len(), 1);
    assert_eq!(runtime.handler_posts[0].runnable, second_runnable);
}

#[test]
fn ui_thread_work_delivers_due_handler_posts_before_frame_callbacks() {
    let program = DexProgram::new(Vec::new());
    let runnable = DexValue::Object(DexObjectRef {
        id: 7,
        class_descriptor: std::rc::Rc::from("Lexample/Runnable;"),
    });
    let frame_callback = DexValue::Object(DexObjectRef {
        id: 8,
        class_descriptor: std::rc::Rc::from("Lexample/FrameCallback;"),
    });

    let mut runtime = program.runtime.borrow_mut();
    runtime.post_handler_runnable("reference:handler".to_owned(), runnable.clone(), 0, false);
    runtime
        .choreographer_frame_callbacks
        .push(frame_callback.clone());

    let work = runtime.take_main_thread_work_with_limit(true, None);
    assert_eq!(work.handler_posts.len(), 1);
    assert_eq!(work.handler_posts[0].runnable, runnable);
    assert!(work.frame_callbacks.is_empty());
    assert_eq!(runtime.choreographer_frame_callbacks, vec![frame_callback]);
}

#[test]
fn choreographer_frame_time_advances_one_vsync_per_batch() {
    let program = DexProgram::new(Vec::new());
    let mut runtime = program.runtime.borrow_mut();

    assert_eq!(runtime.take_frame_time_nanos(), 16_666_666);
    assert_eq!(runtime.take_frame_time_nanos(), 33_333_332);
    assert_eq!(runtime.take_frame_time_nanos(), 49_999_998);
}

#[test]
fn frame_interval_can_be_reconfigured_before_the_clock_starts() {
    let program = DexProgram::new(Vec::new());
    let mut runtime = program.runtime.borrow_mut();

    assert_eq!(runtime.frame_interval_nanos(), 16_666_666);

    runtime.set_frame_interval_nanos(8_333_333);
    assert_eq!(runtime.frame_interval_nanos(), 8_333_333);
    assert_eq!(runtime.take_frame_time_nanos(), 8_333_333);
    assert_eq!(runtime.take_frame_time_nanos(), 16_666_666);
}

#[test]
fn set_frame_interval_does_not_rewind_a_running_clock() {
    let program = DexProgram::new(Vec::new());
    let mut runtime = program.runtime.borrow_mut();

    assert_eq!(runtime.take_frame_time_nanos(), 16_666_666);

    runtime.set_frame_interval_nanos(8_333_333);
    assert_eq!(runtime.take_frame_time_nanos(), 33_333_332);
    assert_eq!(runtime.take_frame_time_nanos(), 41_666_665);
}

#[test]
fn skipped_frame_intervals_jump_the_clock_to_real_elapsed_time() {
    let program = DexProgram::new(Vec::new());
    let mut runtime = program.runtime.borrow_mut();

    runtime.advance_frame_clock_by_skipped_intervals(4);

    assert_eq!(runtime.take_frame_time_nanos(), 16_666_666 * 5);
    assert_eq!(runtime.take_frame_time_nanos(), 16_666_666 * 6);
}

#[test]
fn fixed_frame_rate_config_sets_the_choreographer_interval() {
    let config =
        crate::config::WinedroidConfig::with_frame_rate(crate::config::WinedroidFrameRate::Fixed {
            frames_per_second: 120,
        });
    let program = DexProgram::with_resource_context_and_config(
        Vec::new(),
        None,
        None,
        None,
        None,
        None,
        config,
    );
    let mut runtime = program.runtime.borrow_mut();

    assert_eq!(runtime.frame_interval_nanos(), 8_333_333);
    assert_eq!(runtime.take_frame_time_nanos(), 8_333_333);
}

#[test]
fn frame_time_advances_uptime_for_due_handler_posts() {
    let program = DexProgram::new(Vec::new());
    let runnable = DexValue::Object(DexObjectRef {
        id: 9,
        class_descriptor: std::rc::Rc::from("Lexample/Runnable;"),
    });
    let mut runtime = program.runtime.borrow_mut();

    runtime.post_handler_runnable("reference:handler".to_owned(), runnable.clone(), 16, false);

    let work = runtime.take_main_thread_work_with_limit(true, None);
    assert!(work.handler_posts.is_empty());
    assert_eq!(runtime.main_thread_uptime_millis, 0);

    let frame_time_nanos = runtime.take_frame_time_nanos();
    runtime.advance_main_thread_uptime_to_frame(frame_time_nanos);

    let work = runtime.take_main_thread_work_with_limit(false, None);
    assert_eq!(runtime.main_thread_uptime_millis, 16);
    assert_eq!(work.handler_posts.len(), 1);
    assert_eq!(work.handler_posts[0].runnable, runnable);
}

#[test]
fn idle_loop_can_advance_uptime_to_the_next_timed_event() {
    let program = DexProgram::new(Vec::new());
    let runnable = DexValue::Object(DexObjectRef {
        id: 9,
        class_descriptor: std::rc::Rc::from("Lexample/Runnable;"),
    });
    let mut runtime = program.runtime.borrow_mut();

    runtime.post_handler_runnable("reference:handler".to_owned(), runnable.clone(), 16, false);

    let work = runtime.take_main_thread_work_with_limit(true, None);
    assert!(work.handler_posts.is_empty());
    assert!(runtime.advance_main_thread_uptime_to_next_timed_event(17));

    let work = runtime.take_main_thread_work_with_limit(false, None);
    assert_eq!(runtime.main_thread_uptime_millis, 16);
    assert_eq!(work.handler_posts.len(), 1);
    assert_eq!(work.handler_posts[0].runnable, runnable);
}

#[test]
fn ui_thread_pump_does_not_advance_to_future_handler_posts() {
    let program = DexProgram::new(Vec::new());
    let runnable = DexValue::Object(DexObjectRef {
        id: 9,
        class_descriptor: std::rc::Rc::from("Lexample/Runnable;"),
    });

    program.runtime.borrow_mut().post_handler_runnable(
        "reference:handler".to_owned(),
        runnable.clone(),
        5_000,
        false,
    );

    let dispatched = program
        .dispatch_pending_main_loop(420, 760, 0)
        .expect("future Handler.postDelayed work should not dispatch");

    let runtime = program.runtime.borrow();
    assert_eq!(dispatched, 0);
    assert_eq!(runtime.main_thread_uptime_millis, 0);
    assert_eq!(runtime.handler_posts.len(), 1);
    assert_eq!(runtime.handler_posts[0].runnable, runnable);
    assert_eq!(runtime.handler_posts[0].delay_millis, 5_000);
    assert_eq!(runtime.handler_posts[0].due_time_millis, 5_000);
}

#[test]
fn ui_thread_pump_limit_can_leave_traversal_work_pending() {
    let program = DexProgram::new(Vec::new());
    program.runtime.borrow_mut().request_draw_traversal();

    let dispatched = program
        .dispatch_pending_main_loop_with_budget(420, 760, 0, 0, None)
        .expect("zero pump budget should not execute pending work")
        .dispatched;

    assert_eq!(dispatched, 0);
    assert!(program.runtime.borrow().has_traversal_request());
}

#[test]
fn aosp_viewroot_dotraversal_dispatches_one_traversal_per_driven_frame() {
    let program = DexProgram::new(Vec::new());
    let mut runtime = program.runtime.borrow_mut();

    runtime.begin_driven_frame(0);
    runtime.request_draw_traversal();
    assert!(runtime.has_dispatchable_traversal_request());
    let first = runtime
        .take_traversal_request()
        .expect("first driven frame should consume one traversal");
    assert!(!first.layout_requested);
    assert!(first.draw_requested);

    runtime.request_draw_traversal();
    assert!(runtime.has_traversal_request());
    assert!(!runtime.has_dispatchable_traversal_request());
    assert!(runtime.take_traversal_request().is_none());

    runtime.begin_driven_frame(0);
    assert!(runtime.has_dispatchable_traversal_request());
    let second = runtime
        .take_traversal_request()
        .expect("next driven frame should consume the queued traversal");
    assert!(!second.layout_requested);
    assert!(second.draw_requested);
}

#[test]
fn builtin_view_important_for_accessibility_round_trips() {
    let program = DexProgram::new(Vec::new());
    let set_important_for_accessibility = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "setImportantForAccessibility".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let get_important_for_accessibility = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "getImportantForAccessibility".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let view = DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:accessibility"));

    assert_eq!(
        program.invoke_builtin_method(
            &get_important_for_accessibility,
            Some(&test_invocation(vec![view.clone()]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_important_for_accessibility,
            Some(&test_invocation(vec![view.clone(), DexValue::Int(1)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_important_for_accessibility,
            Some(&test_invocation(vec![view.clone()]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.read_stored_instance_field(&view, &view_important_for_accessibility_field()),
        Some(DexValue::Int(1))
    );
}

#[test]
fn builtin_view_important_for_autofill_round_trips() {
    let program = DexProgram::new(Vec::new());
    let set_important_for_autofill = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "setImportantForAutofill".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let get_important_for_autofill = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "getImportantForAutofill".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let view = DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:autofill"));

    assert_eq!(
        program.invoke_builtin_method(
            &get_important_for_autofill,
            Some(&test_invocation(vec![view.clone()]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_important_for_autofill,
            Some(&test_invocation(vec![view.clone(), DexValue::Int(1)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_important_for_autofill,
            Some(&test_invocation(vec![view.clone()]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.read_stored_instance_field(&view, &view_important_for_autofill_field()),
        Some(DexValue::Int(1))
    );
}

#[test]
fn builtin_view_important_for_content_capture_round_trips() {
    let program = DexProgram::new(Vec::new());
    let set_important_for_content_capture = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "setImportantForContentCapture".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let get_important_for_content_capture = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "getImportantForContentCapture".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let view = DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:content-capture"));

    assert_eq!(
        program.invoke_builtin_method(
            &get_important_for_content_capture,
            Some(&test_invocation(vec![view.clone()]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_important_for_content_capture,
            Some(&test_invocation(vec![view.clone(), DexValue::Int(1)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_important_for_content_capture,
            Some(&test_invocation(vec![view.clone()]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.read_stored_instance_field(&view, &view_important_for_content_capture_field()),
        Some(DexValue::Int(1))
    );
}

#[test]
fn builtin_view_get_content_capture_session_returns_null() {
    let program = DexProgram::new(Vec::new());
    let get_content_capture_session = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "getContentCaptureSession".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/view/contentcapture/ContentCaptureSession;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let view = DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:content-capture"));

    assert_eq!(
        program.invoke_builtin_method(
            &get_content_capture_session,
            Some(&test_invocation(vec![view]))
        ),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
}

#[test]
fn builtin_view_get_autofill_id_is_stable_per_view() {
    let program = DexProgram::new(Vec::new());
    let get_autofill_id = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "getAutofillId".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/view/autofill/AutofillId;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let first_view = DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:first"));
    let second_view = DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:second"));

    let first_id = program.invoke_builtin_method(
        &get_autofill_id,
        Some(&test_invocation(vec![first_view.clone()])),
    );
    let repeated_first_id =
        program.invoke_builtin_method(&get_autofill_id, Some(&test_invocation(vec![first_view])));
    let second_id =
        program.invoke_builtin_method(&get_autofill_id, Some(&test_invocation(vec![second_view])));

    assert_eq!(first_id, repeated_first_id);
    assert_ne!(first_id, second_id);
}

#[test]
fn builtin_view_set_visibility_returns_void() {
    let program = DexProgram::new(Vec::new());
    let set_visibility = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "setVisibility".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let get_visibility = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "getVisibility".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let request_layout = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "requestLayout".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let invalidate = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "invalidate".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let request_apply_insets = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "requestApplyInsets".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let force_layout = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "forceLayout".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let is_layout_requested = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "isLayoutRequested".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let request_focus = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "requestFocus".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let request_focus_with_rect = DexMethodId {
        class_descriptor: "Landroid/view/ViewGroup;".to_owned(),
        name: "requestFocus".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZIL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["I".to_owned(), "Landroid/graphics/Rect;".to_owned()],
        },
    };
    let find_focus = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "findFocus".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/view/View;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let focusable_view_available = DexMethodId {
        class_descriptor: "Landroid/view/ViewGroup;".to_owned(),
        name: "focusableViewAvailable".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/view/View;".to_owned()],
        },
    };
    let awaken_scroll_bars = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "awakenScrollBars".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set_will_not_draw = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "setWillNotDraw".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VZ".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Z".to_owned()],
        },
    };
    let set_enabled = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "setEnabled".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VZ".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Z".to_owned()],
        },
    };
    let set_clickable = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "setClickable".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VZ".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Z".to_owned()],
        },
    };
    let is_clickable = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "isClickable".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set_long_clickable = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "setLongClickable".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VZ".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Z".to_owned()],
        },
    };
    let is_long_clickable = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "isLongClickable".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let on_touch_event = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "onTouchEvent".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Landroid/view/MotionEvent;".to_owned()],
        },
    };
    let set_scroll_container = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "setScrollContainer".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VZ".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Z".to_owned()],
        },
    };
    let is_scroll_container = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "isScrollContainer".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set_save_from_parent_enabled = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "setSaveFromParentEnabled".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VZ".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Z".to_owned()],
        },
    };
    let is_save_from_parent_enabled = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "isSaveFromParentEnabled".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set_focusable = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "setFocusable".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VZ".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Z".to_owned()],
        },
    };
    let set_focusable_mode = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "setFocusable".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let get_focusable = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "getFocusable".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let is_focusable = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "isFocusable".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set_focusable_in_touch_mode = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "setFocusableInTouchMode".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VZ".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Z".to_owned()],
        },
    };
    let is_focusable_in_touch_mode = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "isFocusableInTouchMode".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let is_focused = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "isFocused".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let has_focus = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "hasFocus".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set_default_focus_highlight_enabled = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "setDefaultFocusHighlightEnabled".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VZ".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Z".to_owned()],
        },
    };
    let get_default_focus_highlight_enabled = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "getDefaultFocusHighlightEnabled".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set_force_dark_allowed = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "setForceDarkAllowed".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VZ".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Z".to_owned()],
        },
    };
    let is_force_dark_allowed = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "isForceDarkAllowed".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set_over_scroll_mode = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "setOverScrollMode".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let get_over_scroll_mode = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "getOverScrollMode".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set_layout_direction = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "setLayoutDirection".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let set_keep_screen_on = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "setKeepScreenOn".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VZ".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Z".to_owned()],
        },
    };
    let get_keep_screen_on = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "getKeepScreenOn".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set_layout_params = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "setLayoutParams".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/view/ViewGroup$LayoutParams;".to_owned()],
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
    let generate_default_layout_params = DexMethodId {
        class_descriptor: "Landroid/view/ViewGroup;".to_owned(),
        name: "generateDefaultLayoutParams".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/view/ViewGroup$LayoutParams;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let check_layout_params = DexMethodId {
        class_descriptor: "Landroid/view/ViewGroup;".to_owned(),
        name: "checkLayoutParams".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Landroid/view/ViewGroup$LayoutParams;".to_owned()],
        },
    };
    let margin_layout_params_init = DexMethodId {
        class_descriptor: "Landroid/view/ViewGroup$MarginLayoutParams;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VII".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned(), "I".to_owned()],
        },
    };
    let set_margins = DexMethodId {
        class_descriptor: "Landroid/view/ViewGroup$MarginLayoutParams;".to_owned(),
        name: "setMargins".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VIIII".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "I".to_owned(),
                "I".to_owned(),
                "I".to_owned(),
                "I".to_owned(),
            ],
        },
    };
    let perform_haptic_feedback = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "performHapticFeedback".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZI".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let view = DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:layout/1"));
    let id_backed_view =
        DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:id/2131296715"));
    let force_layout_view =
        DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:force-layout"));
    let layout_direction_view =
        DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:layout-direction"));
    let focus_view_group = DexValue::Reference(std::rc::Rc::from("Landroid/view/ViewGroup;:focus"));
    let motion_event = DexValue::Reference(std::rc::Rc::from("Landroid/view/MotionEvent;:touch"));
    assert!(program.write_stored_instance_field(
        &motion_event,
        &motion_event_action_field(),
        DexValue::Int(1),
    ));

    assert_eq!(
        program.invoke_builtin_method(&get_visibility, Some(&test_invocation(vec![view.clone()]))),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_visibility,
            Some(&test_invocation(vec![view.clone(), DexValue::Int(8)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&get_visibility, Some(&test_invocation(vec![view.clone()]))),
        Some(DexExecutionResult::Int(8))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_visibility,
            Some(&test_invocation(vec![id_backed_view, DexValue::Int(8)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.view_snapshot().view_visibilities,
        vec![DexViewVisibility {
            view_id: 2131296715,
            visibility: 8,
        }]
    );
    assert_eq!(
        program.invoke_builtin_method(&is_focusable, Some(&test_invocation(vec![view.clone()]))),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&is_focused, Some(&test_invocation(vec![view.clone()]))),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&has_focus, Some(&test_invocation(vec![view.clone()]))),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&get_focusable, Some(&test_invocation(vec![view.clone()]))),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_focusable,
            Some(&test_invocation(vec![view.clone(), DexValue::Int(1)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&is_focusable, Some(&test_invocation(vec![view.clone()]))),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&get_focusable, Some(&test_invocation(vec![view.clone()]))),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_focusable_mode,
            Some(&test_invocation(vec![view.clone(), DexValue::Int(16)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&is_focusable, Some(&test_invocation(vec![view.clone()]))),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&get_focusable, Some(&test_invocation(vec![view.clone()]))),
        Some(DexExecutionResult::Int(16))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_focusable_mode,
            Some(&test_invocation(vec![view.clone(), DexValue::Int(0)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&is_focusable, Some(&test_invocation(vec![view.clone()]))),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&get_focusable, Some(&test_invocation(vec![view.clone()]))),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_focusable_in_touch_mode,
            Some(&test_invocation(vec![view.clone(), DexValue::Int(1)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &is_focusable_in_touch_mode,
            Some(&test_invocation(vec![view.clone()]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_default_focus_highlight_enabled,
            Some(&test_invocation(vec![view.clone()]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_default_focus_highlight_enabled,
            Some(&test_invocation(vec![view.clone(), DexValue::Int(0)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_default_focus_highlight_enabled,
            Some(&test_invocation(vec![view.clone()]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &is_force_dark_allowed,
            Some(&test_invocation(vec![view.clone()]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_force_dark_allowed,
            Some(&test_invocation(vec![view.clone(), DexValue::Int(0)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &is_force_dark_allowed,
            Some(&test_invocation(vec![view.clone()]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_over_scroll_mode,
            Some(&test_invocation(vec![view.clone()]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_over_scroll_mode,
            Some(&test_invocation(vec![view.clone(), DexValue::Int(2)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_over_scroll_mode,
            Some(&test_invocation(vec![view.clone()]))
        ),
        Some(DexExecutionResult::Int(2))
    );
    assert_eq!(
        program.read_stored_instance_field(&view, &view_over_scroll_mode_field()),
        Some(DexValue::Int(2))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_layout_direction,
            Some(&test_invocation(vec![
                layout_direction_view.clone(),
                DexValue::Int(1)
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&layout_direction_view, &view_layout_direction_field()),
        Some(DexValue::Int(1))
    );
    assert_eq!(
        program.read_stored_instance_field(&layout_direction_view, &view_layout_requested_field()),
        Some(DexValue::Int(1))
    );
    assert_eq!(
        program.read_stored_instance_field(&layout_direction_view, &view_invalidated_field()),
        Some(DexValue::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_keep_screen_on,
            Some(&test_invocation(vec![view.clone()]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_keep_screen_on,
            Some(&test_invocation(vec![view.clone(), DexValue::Int(1)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_keep_screen_on,
            Some(&test_invocation(vec![view.clone()]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.read_stored_instance_field(&view, &view_keep_screen_on_field()),
        Some(DexValue::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &is_scroll_container,
            Some(&test_invocation(vec![view.clone()]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_scroll_container,
            Some(&test_invocation(vec![view.clone(), DexValue::Int(1)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &is_scroll_container,
            Some(&test_invocation(vec![view.clone()]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.read_stored_instance_field(&view, &view_scroll_container_field()),
        Some(DexValue::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &is_save_from_parent_enabled,
            Some(&test_invocation(vec![view.clone()]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_save_from_parent_enabled,
            Some(&test_invocation(vec![view.clone(), DexValue::Int(0)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &is_save_from_parent_enabled,
            Some(&test_invocation(vec![view.clone()]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.read_stored_instance_field(&view, &view_save_from_parent_enabled_field()),
        Some(DexValue::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_layout_params,
            Some(&test_invocation(vec![view.clone()]))
        ),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    let layout_params = DexValue::Reference(std::rc::Rc::from(
        "Landroid/view/ViewGroup$LayoutParams;:view",
    ));
    assert_eq!(
        program.invoke_builtin_method(
            &set_layout_params,
            Some(&test_invocation(vec![view.clone(), layout_params.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_layout_params,
            Some(&test_invocation(vec![view.clone()]))
        ),
        Some(DexExecutionResult::Value(layout_params))
    );
    let Some(DexExecutionResult::Value(default_layout_params)) =
        program.invoke_builtin_method(&generate_default_layout_params, None)
    else {
        panic!("generateDefaultLayoutParams should return LayoutParams");
    };
    assert_eq!(
        program.read_stored_instance_field(
            &default_layout_params,
            &view_group_layout_params_int_field("width")
        ),
        Some(DexValue::Int(-2))
    );
    assert_eq!(
        program.read_stored_instance_field(
            &default_layout_params,
            &view_group_layout_params_int_field("height")
        ),
        Some(DexValue::Int(-2))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &check_layout_params,
            Some(&test_invocation(vec![
                view.clone(),
                default_layout_params.clone()
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &check_layout_params,
            Some(&test_invocation(vec![view.clone(), DexValue::Null]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    let margin_layout_params = DexValue::Reference(std::rc::Rc::from(
        "Landroid/view/ViewGroup$MarginLayoutParams;:toolbar",
    ));
    assert_eq!(
        program.invoke_builtin_method(
            &margin_layout_params_init,
            Some(&test_invocation(vec![
                margin_layout_params.clone(),
                DexValue::Int(-2),
                DexValue::Int(-1),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(
            &margin_layout_params,
            &view_group_layout_params_int_field("width")
        ),
        Some(DexValue::Int(-2))
    );
    assert_eq!(
        program.read_stored_instance_field(
            &margin_layout_params,
            &view_group_layout_params_int_field("height")
        ),
        Some(DexValue::Int(-1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_margins,
            Some(&test_invocation(vec![
                margin_layout_params.clone(),
                DexValue::Int(1),
                DexValue::Int(2),
                DexValue::Int(3),
                DexValue::Int(4),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(
            &margin_layout_params,
            &view_group_margin_layout_params_int_field("leftMargin")
        ),
        Some(DexValue::Int(1))
    );
    assert_eq!(
        program.read_stored_instance_field(
            &margin_layout_params,
            &view_group_margin_layout_params_int_field("topMargin")
        ),
        Some(DexValue::Int(2))
    );
    assert_eq!(
        program.read_stored_instance_field(
            &margin_layout_params,
            &view_group_margin_layout_params_int_field("rightMargin")
        ),
        Some(DexValue::Int(3))
    );
    assert_eq!(
        program.read_stored_instance_field(
            &margin_layout_params,
            &view_group_margin_layout_params_int_field("bottomMargin")
        ),
        Some(DexValue::Int(4))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &is_layout_requested,
            Some(&test_invocation(vec![view.clone()]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&request_layout, Some(&test_invocation(vec![view.clone()]))),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&invalidate, Some(&test_invocation(vec![view.clone()]))),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&view, &view_invalidated_field()),
        Some(DexValue::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &request_apply_insets,
            Some(&test_invocation(vec![view.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &is_layout_requested,
            Some(&test_invocation(vec![view.clone()]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &force_layout,
            Some(&test_invocation(vec![force_layout_view.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &is_layout_requested,
            Some(&test_invocation(vec![force_layout_view]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&request_focus, Some(&test_invocation(vec![view.clone()]))),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&is_focused, Some(&test_invocation(vec![view.clone()]))),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &request_focus_with_rect,
            Some(&test_invocation(vec![
                focus_view_group.clone(),
                DexValue::Int(130),
                DexValue::Null,
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&is_focused, Some(&test_invocation(vec![focus_view_group]))),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&find_focus, None),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &focusable_view_available,
            Some(&test_invocation(vec![view.clone(), view.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&awaken_scroll_bars, None),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&set_will_not_draw, None),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&set_enabled, None),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&is_clickable, Some(&test_invocation(vec![view.clone()]))),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &on_touch_event,
            Some(&test_invocation(vec![view.clone(), motion_event.clone()]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_clickable,
            Some(&test_invocation(vec![view.clone(), DexValue::Int(1)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&is_clickable, Some(&test_invocation(vec![view.clone()]))),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.read_stored_instance_field(&view, &view_clickable_field()),
        Some(DexValue::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &on_touch_event,
            Some(&test_invocation(vec![view.clone(), motion_event.clone()]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &on_touch_event,
            Some(&test_invocation(vec![view.clone(), DexValue::Null]))
        ),
        None
    );
    assert_eq!(
        program.invoke_builtin_method(
            &is_long_clickable,
            Some(&test_invocation(vec![view.clone()]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_long_clickable,
            Some(&test_invocation(vec![view.clone(), DexValue::Int(1)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &is_long_clickable,
            Some(&test_invocation(vec![view.clone()]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.read_stored_instance_field(&view, &view_long_clickable_field()),
        Some(DexValue::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&perform_haptic_feedback, None),
        Some(DexExecutionResult::Int(0))
    );
}

#[test]
fn builtin_view_request_rectangle_on_screen_returns_false_without_scroll() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "requestRectangleOnScreen".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZLZ".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Landroid/graphics/Rect;".to_owned(), "Z".to_owned()],
        },
    };
    let view = DexValue::Object(DexObjectRef {
        id: 71,
        class_descriptor: std::rc::Rc::from("Landroid/view/View;"),
    });
    let rect = DexValue::Object(DexObjectRef {
        id: 72,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Rect;"),
    });

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![view, rect, DexValue::Int(1)]))
        ),
        Some(DexExecutionResult::Int(0))
    );
}

#[test]
fn builtin_view_group_children_are_tracked() {
    let program = DexProgram::new(Vec::new());
    let group = DexValue::Reference(std::rc::Rc::from("Landroid/view/ViewGroup;:root"));
    let first = DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:first"));
    let second = DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:second"));
    let inserted = DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:inserted"));
    let inserted_layout_params = DexValue::Reference(std::rc::Rc::from(
        "Landroid/view/ViewGroup$LayoutParams;:inserted",
    ));
    let add_view = DexMethodId {
        class_descriptor: "Landroid/view/ViewGroup;".to_owned(),
        name: "addView".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/view/View;".to_owned()],
        },
    };
    let add_view_in_layout = DexMethodId {
        class_descriptor: "Landroid/view/ViewGroup;".to_owned(),
        name: "addViewInLayout".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZLILZ".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec![
                "Landroid/view/View;".to_owned(),
                "I".to_owned(),
                "Landroid/view/ViewGroup$LayoutParams;".to_owned(),
                "Z".to_owned(),
            ],
        },
    };
    let get_child_count = DexMethodId {
        class_descriptor: "Landroid/view/ViewGroup;".to_owned(),
        name: "getChildCount".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get_child_at = DexMethodId {
        class_descriptor: "Landroid/view/ViewGroup;".to_owned(),
        name: "getChildAt".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LI".to_owned(),
            return_type: "Landroid/view/View;".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let remove_all_views = DexMethodId {
        class_descriptor: "Landroid/view/ViewGroup;".to_owned(),
        name: "removeAllViews".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let remove_view_at = DexMethodId {
        class_descriptor: "Landroid/view/ViewGroup;".to_owned(),
        name: "removeViewAt".to_owned(),
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

    assert_eq!(
        program.invoke_builtin_method(
            &add_view,
            Some(&test_invocation(vec![group.clone(), first.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &add_view,
            Some(&test_invocation(vec![group.clone(), second.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &add_view_in_layout,
            Some(&test_invocation(vec![
                group.clone(),
                inserted.clone(),
                DexValue::Int(1),
                inserted_layout_params.clone(),
                DexValue::Int(1)
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_child_count,
            Some(&test_invocation(vec![group.clone()]))
        ),
        Some(DexExecutionResult::Int(3))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_child_at,
            Some(&test_invocation(vec![group.clone(), DexValue::Int(0)]))
        ),
        Some(DexExecutionResult::Value(first))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_child_at,
            Some(&test_invocation(vec![group.clone(), DexValue::Int(1)]))
        ),
        Some(DexExecutionResult::Value(inserted.clone()))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_child_at,
            Some(&test_invocation(vec![group.clone(), DexValue::Int(2)]))
        ),
        Some(DexExecutionResult::Value(second.clone()))
    );
    assert_eq!(
        program.read_stored_instance_field(&inserted, &view_layout_params_field()),
        Some(inserted_layout_params)
    );
    assert_eq!(
        program.invoke_builtin_method(&get_parent, Some(&test_invocation(vec![inserted.clone()]))),
        Some(DexExecutionResult::Value(group.clone()))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &remove_view_at,
            Some(&test_invocation(vec![group.clone(), DexValue::Int(1)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&get_parent, Some(&test_invocation(vec![inserted.clone()]))),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_child_count,
            Some(&test_invocation(vec![group.clone()]))
        ),
        Some(DexExecutionResult::Int(2))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_child_at,
            Some(&test_invocation(vec![group.clone(), DexValue::Int(1)]))
        ),
        Some(DexExecutionResult::Value(second))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &remove_view_at,
            Some(&test_invocation(vec![group.clone(), DexValue::Int(99)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_child_at,
            Some(&test_invocation(vec![group.clone(), DexValue::Int(3)]))
        ),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &remove_all_views,
            Some(&test_invocation(vec![group.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&get_child_count, Some(&test_invocation(vec![group]))),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &remove_all_views,
            Some(&test_invocation(vec![DexValue::Null]))
        ),
        Some(DexExecutionResult::Void)
    );
}

#[test]
fn builtin_view_manager_add_view_attaches_runtime_root() {
    let program = DexProgram::new(Vec::new());
    let add_view = DexMethodId {
        class_descriptor: "Landroid/view/ViewManager;".to_owned(),
        name: "addView".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VLL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "Landroid/view/View;".to_owned(),
                "Landroid/view/ViewGroup$LayoutParams;".to_owned(),
            ],
        },
    };
    let update_view_layout = DexMethodId {
        class_descriptor: "Landroid/view/ViewManager;".to_owned(),
        name: "updateViewLayout".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VLL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "Landroid/view/View;".to_owned(),
                "Landroid/view/ViewGroup$LayoutParams;".to_owned(),
            ],
        },
    };
    let manager = DexValue::Reference(std::rc::Rc::from("Landroid/view/WindowManager;:main"));
    let view = DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:popup"));
    let layout_params = DexValue::Reference(std::rc::Rc::from(
        "Landroid/view/WindowManager$LayoutParams;:popup",
    ));
    let updated_layout_params = DexValue::Reference(std::rc::Rc::from(
        "Landroid/view/WindowManager$LayoutParams;:updated",
    ));

    assert_eq!(
        program.invoke_builtin_method(
            &add_view,
            Some(&test_invocation(vec![
                manager.clone(),
                view.clone(),
                layout_params.clone()
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&view, &view_layout_params_field()),
        Some(layout_params)
    );
    assert_eq!(
        program.view_snapshot().runtime_view_roots,
        vec![DexRuntimeViewRoot {
            view_key: "reference:Landroid/view/View;:popup".to_owned(),
            class_descriptor: "Landroid/view/View;".to_owned(),
        }]
    );
    assert_eq!(
        program.invoke_builtin_method(
            &update_view_layout,
            Some(&test_invocation(vec![
                manager,
                view.clone(),
                updated_layout_params.clone()
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&view, &view_layout_params_field()),
        Some(updated_layout_params)
    );
    assert_eq!(program.view_snapshot().runtime_view_roots.len(), 1);
}

#[test]
fn builtin_view_group_add_view_sets_parent_and_attachment() {
    let program = DexProgram::new(Vec::new());
    let content = view_reference(ANDROID_CONTENT_VIEW_ID);
    let child = DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:compose"));
    let layout_params = DexValue::Reference(std::rc::Rc::from(
        "Landroid/view/ViewGroup$LayoutParams;:compose",
    ));
    let add_view = DexMethodId {
        class_descriptor: "Landroid/view/ViewGroup;".to_owned(),
        name: "addView".to_owned(),
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
    let get_root_view = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "getRootView".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/view/View;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let is_attached = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "isAttachedToWindow".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
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

    assert_eq!(
        program.invoke_builtin_method(&is_attached, Some(&test_invocation(vec![child.clone()]))),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &add_view,
            Some(&test_invocation(vec![
                content.clone(),
                child.clone(),
                layout_params.clone(),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&get_parent, Some(&test_invocation(vec![child.clone()]))),
        Some(DexExecutionResult::Value(content.clone()))
    );
    assert_eq!(
        program.invoke_builtin_method(&get_root_view, Some(&test_invocation(vec![child.clone()]))),
        Some(DexExecutionResult::Value(content))
    );
    assert_eq!(
        program.invoke_builtin_method(&is_attached, Some(&test_invocation(vec![child.clone()]))),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&get_layout_params, Some(&test_invocation(vec![child]))),
        Some(DexExecutionResult::Value(layout_params))
    );
}

#[test]
fn aosp_view_is_hardware_accelerated_requires_attach_info_hardware_window() {
    let program = DexProgram::new(Vec::new());
    let is_hardware_accelerated = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "isHardwareAccelerated".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let root = DexValue::Reference(std::rc::Rc::from("Landroid/view/ViewGroup;:root"));
    let child = DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:child"));

    assert_eq!(
        program.invoke_builtin_method(
            &is_hardware_accelerated,
            Some(&test_invocation(vec![root.clone()]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &is_hardware_accelerated,
            Some(&test_invocation(vec![child.clone()]))
        ),
        Some(DexExecutionResult::Int(0))
    );

    {
        let mut runtime = program.runtime.borrow_mut();
        runtime.runtime_view_roots.push(DexRuntimeViewRoot {
            view_key: map_identity_key(&root),
            class_descriptor: "Landroid/view/ViewGroup;".to_owned(),
        });
        runtime
            .view_parents
            .insert(map_identity_key(&child), root.clone());
    }

    assert_eq!(
        program.invoke_builtin_method(&is_hardware_accelerated, Some(&test_invocation(vec![root]))),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &is_hardware_accelerated,
            Some(&test_invocation(vec![child]))
        ),
        Some(DexExecutionResult::Int(1))
    );
}

#[test]
fn dynamic_container_children_snapshot_materializes_addview_rows_under_xml_container() {
    // cite: frameworks-base ViewGroup.java#addView
    let program = DexProgram::new(Vec::new());
    let layout_id = 0x7f04_0000;
    let container_view_id: i32 = 0x7f03_0005;
    let container = layout_child_reference(
        layout_id,
        container_view_id,
        "Landroid/widget/LinearLayout;",
    );

    let row = DexValue::Object(DexObjectRef {
        id: 1,
        class_descriptor: std::rc::Rc::from("Landroid/widget/LinearLayout;"),
    });
    let image = DexValue::Object(DexObjectRef {
        id: 2,
        class_descriptor: std::rc::Rc::from("Landroid/widget/ImageView;"),
    });
    let label = DexValue::Object(DexObjectRef {
        id: 3,
        class_descriptor: std::rc::Rc::from("Landroid/widget/TextView;"),
    });
    assert!(program.write_stored_instance_field(
        &label,
        &text_view_text_field(),
        DexValue::String(std::rc::Rc::from("notes.txt")),
    ));
    // cite: frameworks-base View.java#setMinimumWidth, setMinimumHeight
    assert!(program.write_stored_instance_field(
        &image,
        &view_minimum_width_field(),
        DexValue::Int(52),
    ));
    assert!(program.write_stored_instance_field(
        &image,
        &view_minimum_height_field(),
        DexValue::Int(52),
    ));

    assert!(program
        .view_group_add_view(&test_invocation(vec![row.clone(), image.clone()]))
        .is_some());
    assert!(program
        .view_group_add_view(&test_invocation(vec![row.clone(), label.clone()]))
        .is_some());
    assert!(program
        .view_group_add_view(&test_invocation(vec![container, row]))
        .is_some());

    let snapshot = program.dynamic_container_children_snapshot();
    assert_eq!(snapshot.len(), 1);
    let group = &snapshot[0];
    assert_eq!(group.container_view_id, container_view_id as u32);
    assert_eq!(group.children.len(), 1);
    let row_node = &group.children[0];
    assert_eq!(row_node.tag_name, "LinearLayout");
    assert_eq!(row_node.children.len(), 2);
    assert_eq!(row_node.children[0].tag_name, "ImageView");
    assert_eq!(row_node.children[0].explicit_width, Some(52));
    assert_eq!(row_node.children[0].explicit_height, Some(52));
    assert_eq!(row_node.children[1].tag_name, "TextView");
    assert_eq!(row_node.children[1].text.as_deref(), Some("notes.txt"));
}

#[test]
fn dynamic_container_children_snapshot_mints_a_click_id_preserving_the_real_receiver_class() {
    // cite: frameworks-base View.java#setOnClickListener, performClick
    let program = DexProgram::new(Vec::new());
    let layout_id = 0x7f04_0000;
    let container_view_id: i32 = 0x7f03_0005;
    let container = layout_child_reference(
        layout_id,
        container_view_id,
        "Landroid/widget/LinearLayout;",
    );
    let row = DexValue::Object(DexObjectRef {
        id: 42,
        class_descriptor: std::rc::Rc::from("Lc/b;"),
    });
    let listener = DexValue::Object(DexObjectRef {
        id: 43,
        class_descriptor: std::rc::Rc::from(
            "Lcom/martinmimigames/simplefileexplorer/MainActivity$e;",
        ),
    });
    let set_on_click_listener = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "setOnClickListener".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/view/View$OnClickListener;".to_owned()],
        },
    };
    assert_eq!(
        program.invoke_builtin_method(
            &set_on_click_listener,
            Some(&test_invocation(vec![row.clone(), listener]))
        ),
        Some(DexExecutionResult::Void)
    );

    assert!(program
        .view_group_add_view(&test_invocation(vec![container, row.clone()]))
        .is_some());

    let snapshot = program.dynamic_container_children_snapshot();
    let click_id = snapshot[0].children[0]
        .click_id
        .expect("row with a click listener should get a synthetic click id");

    let resolved = program
        .dynamic_view_click_target(click_id)
        .expect("click id should resolve back to the row object");
    assert_eq!(resolved, row);
}

#[test]
fn builtin_color_drawable_constructor_stores_color() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Landroid/graphics/drawable/ColorDrawable;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let drawable = DexValue::Object(DexObjectRef {
        id: 42,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/drawable/ColorDrawable;"),
    });

    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![drawable.clone(), DexValue::Int(-49)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&drawable, &color_drawable_color_field()),
        Some(DexValue::Int(-49))
    );
}

#[test]
fn builtin_drawable_set_visible_returns_whether_visibility_changed() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/graphics/drawable/Drawable;".to_owned(),
        name: "setVisible".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZZZ".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Z".to_owned(), "Z".to_owned()],
        },
    };
    let drawable = DexValue::Object(DexObjectRef {
        id: 127,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/drawable/ColorDrawable;"),
    });

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                drawable.clone(),
                DexValue::Int(0),
                DexValue::Int(0)
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.read_stored_instance_field(&drawable, &drawable_visible_field()),
        Some(DexValue::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                drawable.clone(),
                DexValue::Int(0),
                DexValue::Int(1)
            ]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                drawable,
                DexValue::Int(1),
                DexValue::Int(1)
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
}

#[test]
fn builtin_ripple_drawable_constructor_stores_layers() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Landroid/graphics/drawable/RippleDrawable;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VLLL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "Landroid/content/res/ColorStateList;".to_owned(),
                "Landroid/graphics/drawable/Drawable;".to_owned(),
                "Landroid/graphics/drawable/Drawable;".to_owned(),
            ],
        },
    };
    let get_radius = DexMethodId {
        class_descriptor: "Landroid/graphics/drawable/RippleDrawable;".to_owned(),
        name: "getRadius".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set_radius = DexMethodId {
        class_descriptor: "Landroid/graphics/drawable/RippleDrawable;".to_owned(),
        name: "setRadius".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let set_color = DexMethodId {
        class_descriptor: "Landroid/graphics/drawable/RippleDrawable;".to_owned(),
        name: "setColor".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/content/res/ColorStateList;".to_owned()],
        },
    };
    let ripple = DexValue::Object(DexObjectRef {
        id: 43,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/drawable/RippleDrawable;"),
    });
    let color = DexValue::Reference(std::rc::Rc::from(
        "Landroid/content/res/ColorStateList;:ripple",
    ));
    let content = DexValue::Reference(std::rc::Rc::from(
        "Landroid/graphics/drawable/Drawable;:content",
    ));
    let mask = DexValue::Reference(std::rc::Rc::from(
        "Landroid/graphics/drawable/Drawable;:mask",
    ));

    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![
                ripple.clone(),
                color.clone(),
                content.clone(),
                mask.clone()
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&ripple, &ripple_drawable_color_field()),
        Some(color)
    );
    assert_eq!(
        program.read_stored_instance_field(&ripple, &ripple_drawable_content_field()),
        Some(content)
    );
    assert_eq!(
        program.read_stored_instance_field(&ripple, &ripple_drawable_mask_field()),
        Some(mask)
    );
    assert_eq!(
        program.invoke_builtin_method(&get_radius, Some(&test_invocation(vec![ripple.clone()]))),
        Some(DexExecutionResult::Int(-1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_radius,
            Some(&test_invocation(vec![ripple.clone(), DexValue::Int(24)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&get_radius, Some(&test_invocation(vec![ripple.clone()]))),
        Some(DexExecutionResult::Int(24))
    );

    let updated_color = DexValue::Reference(std::rc::Rc::from(
        "Landroid/content/res/ColorStateList;:updated-ripple",
    ));
    assert_eq!(
        program.invoke_builtin_method(
            &set_color,
            Some(&test_invocation(vec![
                ripple.clone(),
                updated_color.clone()
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&ripple, &ripple_drawable_color_field()),
        Some(updated_color)
    );
}

#[test]
fn builtin_view_set_minimum_width_and_height_store_values() {
    // cite: frameworks-base View.java#setMinimumWidth, setMinimumHeight
    let program = DexProgram::new(Vec::new());
    let set_minimum_width = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "setMinimumWidth".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let set_minimum_height = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "setMinimumHeight".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let view = DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:icon"));

    assert_eq!(
        program.invoke_builtin_method(
            &set_minimum_width,
            Some(&test_invocation(vec![view.clone(), DexValue::Int(420)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_minimum_height,
            Some(&test_invocation(vec![view.clone(), DexValue::Int(240)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&view, &view_minimum_width_field()),
        Some(DexValue::Int(420))
    );
    assert_eq!(
        program.read_stored_instance_field(&view, &view_minimum_height_field()),
        Some(DexValue::Int(240))
    );
}

#[test]
fn builtin_view_set_background_color_stores_color_drawable() {
    // cite: frameworks-base View.java#setBackgroundColor
    let program = DexProgram::new(Vec::new());
    let set_background_color = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "setBackgroundColor".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let get_background = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "getBackground".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/graphics/drawable/Drawable;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let view = DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:row"));

    assert_eq!(
        program.invoke_builtin_method(
            &set_background_color,
            Some(&test_invocation(vec![view.clone(), DexValue::Int(-1)]))
        ),
        Some(DexExecutionResult::Void)
    );
    let Some(DexExecutionResult::Value(drawable)) =
        program.invoke_builtin_method(&get_background, Some(&test_invocation(vec![view])))
    else {
        panic!("getBackground should return the stored ColorDrawable");
    };
    assert_eq!(
        program.read_stored_instance_field(&drawable, &color_drawable_color_field()),
        Some(DexValue::Int(-1))
    );
}

#[test]
fn builtin_view_background_round_trips_drawable() {
    let program = DexProgram::new(Vec::new());
    let set_background = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "setBackground".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/graphics/drawable/Drawable;".to_owned()],
        },
    };
    let get_background = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "getBackground".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/graphics/drawable/Drawable;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set_background_tint_list = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "setBackgroundTintList".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/content/res/ColorStateList;".to_owned()],
        },
    };
    let get_background_tint_list = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "getBackgroundTintList".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/content/res/ColorStateList;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set_background_tint_mode = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "setBackgroundTintMode".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/graphics/PorterDuff$Mode;".to_owned()],
        },
    };
    let get_background_tint_mode = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "getBackgroundTintMode".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/graphics/PorterDuff$Mode;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set_content_description = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "setContentDescription".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/lang/CharSequence;".to_owned()],
        },
    };
    let get_content_description = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "getContentDescription".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/CharSequence;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set_tooltip_text = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "setTooltipText".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/lang/CharSequence;".to_owned()],
        },
    };
    let get_tooltip_text = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "getTooltipText".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/CharSequence;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let view = DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:layout/1"));
    let drawable = DexValue::Reference(std::rc::Rc::from(
        "Landroid/graphics/drawable/Drawable;:resource:1",
    ));

    assert_eq!(
        program.invoke_builtin_method(&get_background, Some(&test_invocation(vec![view.clone()]))),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_background,
            Some(&test_invocation(vec![view.clone(), drawable.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&get_background, Some(&test_invocation(vec![view]))),
        Some(DexExecutionResult::Value(drawable))
    );
    let tinted_view = DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:layout/3"));
    let tint = DexValue::Reference(std::rc::Rc::from(
        "Landroid/content/res/ColorStateList;:accent",
    ));
    assert_eq!(
        program.invoke_builtin_method(
            &get_background_tint_list,
            Some(&test_invocation(vec![tinted_view.clone()]))
        ),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_background_tint_list,
            Some(&test_invocation(vec![tinted_view.clone(), tint.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_background_tint_list,
            Some(&test_invocation(vec![tinted_view.clone()]))
        ),
        Some(DexExecutionResult::Value(tint))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_background_tint_list,
            Some(&test_invocation(vec![
                tinted_view.clone(),
                DexValue::Int(0)
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_background_tint_list,
            Some(&test_invocation(vec![tinted_view.clone()]))
        ),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    let mode = DexValue::Reference(std::rc::Rc::from(
        "Landroid/graphics/PorterDuff$Mode;:SRC_IN",
    ));
    assert_eq!(
        program.invoke_builtin_method(
            &get_background_tint_mode,
            Some(&test_invocation(vec![tinted_view.clone()]))
        ),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_background_tint_mode,
            Some(&test_invocation(vec![tinted_view.clone(), mode.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_background_tint_mode,
            Some(&test_invocation(vec![tinted_view.clone()]))
        ),
        Some(DexExecutionResult::Value(mode))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_background_tint_mode,
            Some(&test_invocation(vec![
                tinted_view.clone(),
                DexValue::Int(0)
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_background_tint_mode,
            Some(&test_invocation(vec![tinted_view]))
        ),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    let described_view = DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:layout/2"));
    assert_eq!(
        program.invoke_builtin_method(
            &get_content_description,
            Some(&test_invocation(vec![described_view.clone()]))
        ),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_content_description,
            Some(&test_invocation(vec![
                described_view.clone(),
                DexValue::String(std::rc::Rc::from("Menu"))
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_content_description,
            Some(&test_invocation(vec![described_view.clone()]))
        ),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("Menu")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_tooltip_text,
            Some(&test_invocation(vec![described_view.clone()]))
        ),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_tooltip_text,
            Some(&test_invocation(vec![
                described_view.clone(),
                DexValue::String(std::rc::Rc::from("Open menu"))
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_tooltip_text,
            Some(&test_invocation(vec![described_view]))
        ),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("Open menu")
        )))
    );
}

// cite: frameworks-base View.java#setForeground, mForegroundInfo
#[test]
fn frameworks_view_set_foreground_null_is_noop_without_foreground() {
    let program = DexProgram::new(Vec::new());
    let set_foreground = DexMethodId {
        class_descriptor: "Landroid/widget/FrameLayout;".to_owned(),
        name: "setForeground".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/graphics/drawable/Drawable;".to_owned()],
        },
    };
    let frame_layout =
        DexValue::Reference(std::rc::Rc::from("Landroid/widget/FrameLayout;:content"));

    assert_eq!(
        program.invoke_builtin_method_in_hierarchy(
            &set_foreground,
            &test_invocation(vec![frame_layout.clone(), DexValue::Int(0)])
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method_in_hierarchy(
            &set_foreground,
            &test_invocation(vec![
                frame_layout,
                DexValue::Reference(std::rc::Rc::from(
                    "Landroid/graphics/drawable/Drawable;:foreground"
                ))
            ])
        ),
        None
    );
}

#[test]
fn builtin_view_set_id_stores_view_id() {
    let program = DexProgram::new(Vec::new());
    let set_id = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "setId".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let get_id = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "getId".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let view = DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:layout/1"));
    let found_view = DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:id/2131296365"));
    let set_invocation = test_invocation(vec![view.clone(), DexValue::Int(2131296428)]);
    let get_invocation = test_invocation(vec![view]);
    let found_invocation = test_invocation(vec![found_view]);

    assert_eq!(
        program.invoke_builtin_method(&get_id, Some(&get_invocation)),
        Some(DexExecutionResult::Int(-1))
    );
    assert_eq!(
        program.invoke_builtin_method(&get_id, Some(&found_invocation)),
        Some(DexExecutionResult::Int(2131296365))
    );
    assert_eq!(
        program.invoke_builtin_method(&set_id, Some(&set_invocation)),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&get_id, Some(&get_invocation)),
        Some(DexExecutionResult::Int(2131296428))
    );
}

#[test]
// cite: frameworks-base View.java#findViewById, findViewTraversal
fn builtin_view_find_view_by_id_returns_null_when_id_is_absent() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "findViewById".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LI".to_owned(),
            return_type: "Landroid/view/View;".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let invocation = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:layout/1")),
        DexValue::Int(2131427805),
    ]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    assert_eq!(program.view_snapshot().view_lookup_ids, vec![2131427805]);
}

#[test]
fn builtin_view_find_view_by_id_returns_registered_inflated_child() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "findViewById".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LI".to_owned(),
            return_type: "Landroid/view/View;".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let root = DexValue::Reference(std::rc::Rc::from(
        "Landroidx/appcompat/view/menu/ListMenuItemView;:layout/1",
    ));
    let child = DexValue::Reference(std::rc::Rc::from(
        "Landroid/widget/TextView;:layout/1/id/16908310",
    ));

    program
        .runtime
        .borrow_mut()
        .inflated_view_children
        .insert((map_identity_key(&root), 16908310), child.clone());

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![root, DexValue::Int(16908310)]))
        ),
        Some(DexExecutionResult::Value(child))
    );
}

#[test]
fn builtin_view_find_view_by_id_returns_active_layout_child_for_decor_receiver() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "findViewById".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LI".to_owned(),
            return_type: "Landroid/view/View;".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let root = layout_reference(42);
    let child = layout_child_reference(42, 2131296322, "Landroidx/appcompat/widget/Toolbar;");

    {
        let mut runtime = program.runtime.borrow_mut();
        runtime.content_layouts.push(42);
        runtime
            .inflated_view_children
            .insert((map_identity_key(&root), 2131296322), child.clone());
        runtime.activity_stack.push(RuntimeActivity {
            class_descriptor: "Lexample/SettingsActivity;".to_owned(),
            receiver: DexValue::Reference(std::rc::Rc::from("Lexample/SettingsActivity;:activity")),
            method_signature: "onCreate(Landroid/os/Bundle;)V".to_owned(),
            result: "void".to_owned(),
            object_start: 0,
            content_layout_start: 0,
            inflated_layout_start: 0,
            initial_inflated_layout_end: 0,
            dialog_content_layout_start: 0,
            runtime_view_root_start: 0,
            view_lookup_start: 0,
            saved_chrome: None,
            result_target: None,
            result_code: 0,
            result_intent: None,
            adapter_rows_snapshot: std::collections::HashMap::new(),
            adapter_item_clicks_snapshot: std::collections::HashMap::new(),
        });
    }

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::Reference(std::rc::Rc::from(
                    "Lcom/android/internal/policy/DecorView;.decor"
                )),
                DexValue::Int(2131296322),
            ]))
        ),
        Some(DexExecutionResult::Value(child))
    );
}

#[test]
fn builtin_does_not_intercept_obfuscated_view_binding_lookup() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Lq1/b;".to_owned(),
        name: "g".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LLI".to_owned(),
            return_type: "Landroid/view/View;".to_owned(),
            parameters: vec!["Landroid/view/View;".to_owned(), "I".to_owned()],
        },
    };
    let invocation = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:layout/2131492893")),
        DexValue::Int(2131296342),
    ]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        None
    );
}

#[test]
fn builtin_cursor_anchor_info_builder_constructor_accepts_builder_receiver() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Landroid/view/inputmethod/CursorAnchorInfo$Builder;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let builder = DexValue::Object(DexObjectRef {
        id: 31,
        class_descriptor: std::rc::Rc::from("Landroid/view/inputmethod/CursorAnchorInfo$Builder;"),
    });

    assert_eq!(
        program.invoke_builtin_method(&constructor, Some(&test_invocation(vec![builder]))),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(program.invoke_builtin_method(&constructor, None), None);
}

// cite: frameworks-base View.java#setSystemUiVisibility, getSystemUiVisibility
#[test]
fn view_system_ui_visibility_defaults_to_zero_and_round_trips() {
    let program = DexProgram::new(Vec::new());
    let view = program.allocate_object("Landroid/view/View;".to_owned());
    let getter = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "getSystemUiVisibility".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let setter = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "setSystemUiVisibility".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&getter, Some(&test_invocation(vec![view.clone()]))),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &setter,
            Some(&test_invocation(vec![view.clone(), DexValue::Int(0x500)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&getter, Some(&test_invocation(vec![view]))),
        Some(DexExecutionResult::Int(0x500))
    );
}

// cite: frameworks-base ViewGroup.java#requestDisallowInterceptTouchEvent
#[test]
fn request_disallow_intercept_touch_event_stores_flag() {
    let program = DexProgram::new(Vec::new());
    let parent = program.allocate_object("Landroid/view/ViewGroup;".to_owned());
    let method = DexMethodId {
        class_descriptor: "Landroid/view/ViewParent;".to_owned(),
        name: "requestDisallowInterceptTouchEvent".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VZ".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Z".to_owned()],
        },
    };
    let field = crate::dex::DexFieldId {
        class_descriptor: "Landroid/view/ViewGroup;".to_owned(),
        type_descriptor: "Z".to_owned(),
        name: "disallowIntercept".to_owned(),
    };

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![parent.clone(), DexValue::Int(1)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&parent, &field),
        Some(DexValue::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![parent.clone(), DexValue::Int(0)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&parent, &field),
        Some(DexValue::Int(0))
    );
}

#[test]
fn builtin_viewgroup_hierarchy_change_listener_stores_and_clears() {
    // cite: cts TableLayoutTest.java#testSetOnHierarchyChangeListener
    // cite: frameworks-base ViewGroup.java#setOnHierarchyChangeListener
    let program = DexProgram::new(Vec::new());
    let set_listener = DexMethodId {
        class_descriptor: "Landroid/view/ViewGroup;".to_owned(),
        name: "setOnHierarchyChangeListener".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/view/ViewGroup$OnHierarchyChangeListener;".to_owned()],
        },
    };
    let add_view = DexMethodId {
        class_descriptor: "Landroid/view/ViewGroup;".to_owned(),
        name: "addView".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/view/View;".to_owned()],
        },
    };
    let group = DexValue::Reference(std::rc::Rc::from("Landroid/view/ViewGroup;:table"));
    let child = DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:text"));
    let listener = DexValue::Reference(std::rc::Rc::from(
        "Landroid/view/ViewGroup$OnHierarchyChangeListener;:test",
    ));

    assert_eq!(
        program.invoke_builtin_method(
            &set_listener,
            Some(&test_invocation(vec![group.clone(), listener.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&group, &view_hierarchy_change_listener_field()),
        Some(listener)
    );

    assert_eq!(
        program.invoke_builtin_method(
            &set_listener,
            Some(&test_invocation(vec![group.clone(), DexValue::Null]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&group, &view_hierarchy_change_listener_field()),
        Some(DexValue::Null)
    );

    assert_eq!(
        program.invoke_builtin_method(
            &add_view,
            Some(&test_invocation(vec![group.clone(), child.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.runtime.borrow().view_group_children[&map_identity_key(&group)],
        vec![child]
    );
}

#[test]
fn viewgroup_add_view_dispatch_errors_when_listener_callback_is_missing() {
    // cite: cts TableLayoutTest.java#testSetOnHierarchyChangeListener
    // cite: frameworks-base ViewGroup.java#dispatchViewAdded
    let program = DexProgram::new(Vec::new());
    let set_listener = DexMethodId {
        class_descriptor: "Landroid/view/ViewGroup;".to_owned(),
        name: "setOnHierarchyChangeListener".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/view/ViewGroup$OnHierarchyChangeListener;".to_owned()],
        },
    };
    let group = DexValue::Reference(std::rc::Rc::from("Landroid/view/ViewGroup;:table"));
    let child = DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:text"));
    let listener = DexValue::Object(DexObjectRef {
        id: 11,
        class_descriptor: std::rc::Rc::from("Lexample/HierarchyListener;"),
    });

    assert_eq!(
        program.invoke_builtin_method(
            &set_listener,
            Some(&test_invocation(vec![group.clone(), listener]))
        ),
        Some(DexExecutionResult::Void)
    );

    assert_eq!(
        program.notify_view_hierarchy_change(&group, &child, "onChildViewAdded"),
        None
    );
}

#[test]
fn builtin_relative_layout_params_two_arg_constructor_stores_width_and_height() {
    // cite: frameworks-base RelativeLayout.java#LayoutParams
    let program = DexProgram::new(Vec::new());
    let init = DexMethodId {
        class_descriptor: "Landroid/widget/RelativeLayout$LayoutParams;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VII".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned(), "I".to_owned()],
        },
    };
    let layout_params = DexValue::Reference(std::rc::Rc::from(
        "Landroid/widget/RelativeLayout$LayoutParams;:row",
    ));

    assert_eq!(
        program.invoke_builtin_method(
            &init,
            Some(&test_invocation(vec![
                layout_params.clone(),
                DexValue::Int(-1),
                DexValue::Int(-2),
            ]))
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
        Some(DexValue::Int(-2))
    );
}

#[test]
fn builtin_frame_layout_set_foreground_gravity_stores_value_and_requests_layout() {
    // cite: frameworks-base FrameLayout.java#setForegroundGravity
    let program = DexProgram::new(Vec::new());
    let set_foreground_gravity = DexMethodId {
        class_descriptor: "Landroid/widget/FrameLayout;".to_owned(),
        name: "setForegroundGravity".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let layout = DexValue::Reference(std::rc::Rc::from("Landroid/widget/FrameLayout;:card"));
    const CENTER: i32 = 17;

    assert_eq!(
        program.invoke_builtin_method(
            &set_foreground_gravity,
            Some(&test_invocation(vec![
                layout.clone(),
                DexValue::Int(CENTER)
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&layout, &frame_layout_foreground_gravity_field()),
        Some(DexValue::Int(CENTER))
    );
}

#[test]
fn builtin_linear_layout_params_three_arg_constructor_stores_size_and_weight() {
    // cite: frameworks-base LinearLayout.java#LayoutParams
    let program = DexProgram::new(Vec::new());
    let init = DexMethodId {
        class_descriptor: "Landroid/widget/LinearLayout$LayoutParams;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VIIF".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned(), "I".to_owned(), "F".to_owned()],
        },
    };
    let layout_params = DexValue::Reference(std::rc::Rc::from(
        "Landroid/widget/LinearLayout$LayoutParams;:row",
    ));

    assert_eq!(
        program.invoke_builtin_method(
            &init,
            Some(&test_invocation(vec![
                layout_params.clone(),
                DexValue::Int(0),
                DexValue::Int(-2),
                DexValue::Int(1.0_f32.to_bits() as i32),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(
            &layout_params,
            &view_group_layout_params_int_field("width")
        ),
        Some(DexValue::Int(0))
    );
    assert_eq!(
        program.read_stored_instance_field(
            &layout_params,
            &view_group_layout_params_int_field("height")
        ),
        Some(DexValue::Int(-2))
    );
    assert_eq!(
        program.read_stored_instance_field(&layout_params, &linear_layout_params_weight_field()),
        Some(DexValue::Float(1.0))
    );
}

#[test]
fn builtin_linear_layout_set_orientation_stores_value_and_requests_layout() {
    // cite: frameworks-base LinearLayout.java#setOrientation
    let program = DexProgram::new(Vec::new());
    let set_orientation = DexMethodId {
        class_descriptor: "Landroid/widget/LinearLayout;".to_owned(),
        name: "setOrientation".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let layout = DexValue::Reference(std::rc::Rc::from("Landroid/widget/LinearLayout;:row"));
    const VERTICAL: i32 = 1;

    assert_eq!(
        program.invoke_builtin_method(
            &set_orientation,
            Some(&test_invocation(vec![
                layout.clone(),
                DexValue::Int(VERTICAL)
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&layout, &linear_layout_orientation_field()),
        Some(DexValue::Int(VERTICAL))
    );
}

#[test]
fn builtin_linear_layout_set_gravity_stores_value_and_requests_layout() {
    // cite: frameworks-base LinearLayout.java#setGravity
    let program = DexProgram::new(Vec::new());
    let set_gravity = DexMethodId {
        class_descriptor: "Landroid/widget/LinearLayout;".to_owned(),
        name: "setGravity".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let layout = DexValue::Reference(std::rc::Rc::from("Landroid/widget/LinearLayout;:row"));
    const BOTTOM: i32 = 80;

    assert_eq!(
        program.invoke_builtin_method(
            &set_gravity,
            Some(&test_invocation(vec![
                layout.clone(),
                DexValue::Int(BOTTOM)
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&layout, &linear_layout_gravity_field()),
        Some(DexValue::Int(BOTTOM))
    );
}

#[test]
fn builtin_linear_layout_params_two_arg_constructor_stores_width_and_height() {
    // cite: frameworks-base LinearLayout.java#LayoutParams
    let program = DexProgram::new(Vec::new());
    let init = DexMethodId {
        class_descriptor: "Landroid/widget/LinearLayout$LayoutParams;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VII".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned(), "I".to_owned()],
        },
    };
    let layout_params = DexValue::Reference(std::rc::Rc::from(
        "Landroid/widget/LinearLayout$LayoutParams;:row",
    ));

    assert_eq!(
        program.invoke_builtin_method(
            &init,
            Some(&test_invocation(vec![
                layout_params.clone(),
                DexValue::Int(-1),
                DexValue::Int(40),
            ]))
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
        Some(DexValue::Int(40))
    );
}

#[test]
fn builtin_relative_layout_params_add_rule_stores_rule_subject() {
    // cite: frameworks-base RelativeLayout.java#addRule
    let program = DexProgram::new(Vec::new());
    let add_rule = DexMethodId {
        class_descriptor: "Landroid/widget/RelativeLayout$LayoutParams;".to_owned(),
        name: "addRule".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VII".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned(), "I".to_owned()],
        },
    };
    let layout_params = DexValue::Reference(std::rc::Rc::from(
        "Landroid/widget/RelativeLayout$LayoutParams;:row",
    ));
    const CENTER_IN_PARENT: i32 = 13;
    const TRUE: i32 = -1;

    assert_eq!(
        program.invoke_builtin_method(
            &add_rule,
            Some(&test_invocation(vec![
                layout_params.clone(),
                DexValue::Int(CENTER_IN_PARENT),
                DexValue::Int(TRUE),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(
            &layout_params,
            &relative_layout_params_rule_field(CENTER_IN_PARENT)
        ),
        Some(DexValue::Int(TRUE))
    );
}

#[test]
fn builtin_view_is_shown_walks_ancestors_for_visibility_and_attachment() {
    // cite: frameworks-base View.java#isShown
    let program = DexProgram::new(Vec::new());
    let is_shown = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "isShown".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let parent = DexValue::Reference(std::rc::Rc::from("Landroid/widget/LinearLayout;:parent"));
    let attached_child =
        DexValue::Reference(std::rc::Rc::from("Landroid/widget/TextView;:attached"));
    let hidden_child = DexValue::Reference(std::rc::Rc::from("Landroid/widget/TextView;:hidden"));
    let detached = DexValue::Reference(std::rc::Rc::from("Landroid/widget/TextView;:detached"));
    const GONE: i32 = 8;

    {
        let mut runtime = program.runtime.borrow_mut();
        runtime
            .view_parents
            .insert(map_identity_key(&attached_child), parent.clone());
        runtime
            .view_parents
            .insert(map_identity_key(&hidden_child), parent.clone());
        runtime
            .view_parents
            .insert(map_identity_key(&parent), decor_view_reference());
    }
    assert!(program.write_stored_instance_field(
        &hidden_child,
        &view_visibility_field(),
        DexValue::Int(GONE),
    ));

    assert_eq!(
        program.invoke_builtin_method(&is_shown, Some(&test_invocation(vec![attached_child]))),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&is_shown, Some(&test_invocation(vec![hidden_child]))),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&is_shown, Some(&test_invocation(vec![detached]))),
        Some(DexExecutionResult::Int(0))
    );
}

#[test]
fn builtin_view_group_remove_view_removes_by_reference() {
    // cite: frameworks-base ViewGroup.java#removeView, removeViewInternal
    let program = DexProgram::new(Vec::new());
    let add_view = DexMethodId {
        class_descriptor: "Landroid/view/ViewGroup;".to_owned(),
        name: "addView".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/view/View;".to_owned()],
        },
    };
    let remove_view = DexMethodId {
        class_descriptor: "Landroid/view/ViewGroup;".to_owned(),
        name: "removeView".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/view/View;".to_owned()],
        },
    };
    let get_child_count = DexMethodId {
        class_descriptor: "Landroid/view/ViewGroup;".to_owned(),
        name: "getChildCount".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
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
    let group = DexValue::Reference(std::rc::Rc::from("Landroid/widget/LinearLayout;:row"));
    let first = DexValue::Reference(std::rc::Rc::from("Landroid/widget/TextView;:first"));
    let second = DexValue::Reference(std::rc::Rc::from("Landroid/widget/TextView;:second"));

    assert_eq!(
        program.invoke_builtin_method(
            &add_view,
            Some(&test_invocation(vec![group.clone(), first.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &add_view,
            Some(&test_invocation(vec![group.clone(), second.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &remove_view,
            Some(&test_invocation(vec![group.clone(), first.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&get_child_count, Some(&test_invocation(vec![group]))),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&get_parent, Some(&test_invocation(vec![first]))),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    assert_eq!(
        program.invoke_builtin_method(&get_parent, Some(&test_invocation(vec![second]))),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Landroid/widget/LinearLayout;:row")
        )))
    );
}

fn view_group_pressed_state_override_dex(
    platform_superclass: &str,
    override_result: i32,
) -> Vec<u8> {
    const DEX_HEADER_SIZE: usize = 0x70;
    const DEX_HEADER_SIZE_U32: u32 = 0x70;
    const DEX_ENDIAN_CONSTANT: u32 = 0x1234_5678;
    const STRING_ID_SIZE: usize = 4;
    const TYPE_ID_SIZE: usize = 4;
    const PROTO_ID_SIZE: usize = 12;
    const METHOD_ID_SIZE: usize = 8;
    const CLASS_DEF_SIZE: usize = 32;

    let strings = [
        "Lcom/example/DelayGroup;",
        platform_superclass,
        "Z",
        "shouldDelayChildPressedState",
    ];
    let string_ids_off = DEX_HEADER_SIZE;
    let type_ids_off = string_ids_off + STRING_ID_SIZE * strings.len();
    let proto_ids_off = type_ids_off + TYPE_ID_SIZE * 3;
    let method_ids_off = proto_ids_off + PROTO_ID_SIZE;
    let class_defs_off = method_ids_off + METHOD_ID_SIZE * 2;
    let data_off = class_defs_off + CLASS_DEF_SIZE;
    let mut data = vec![0; data_off];
    let mut string_offsets = Vec::new();

    for value in strings {
        string_offsets.push(data.len() as u32);
        write_test_uleb128(&mut data, value.len() as u32);
        data.extend_from_slice(value.as_bytes());
        data.push(0);
    }

    let class_data_off = data.len() as u32;
    write_test_uleb128(&mut data, 0);
    write_test_uleb128(&mut data, 0);
    write_test_uleb128(&mut data, 0);
    write_test_uleb128(&mut data, 1);
    write_test_uleb128(&mut data, 0);
    write_test_uleb128(&mut data, JAVA_ACC_PUBLIC);
    let code_off_index = data.len();
    data.extend_from_slice(&[0, 0]);

    while data.len() % 4 != 0 {
        data.push(0);
    }
    let code_off = data.len() as u32;
    assert!((0x80..0x4000).contains(&code_off));
    data[code_off_index] = ((code_off & 0x7f) as u8) | 0x80;
    data[code_off_index + 1] = (code_off >> 7) as u8;

    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&2u32.to_le_bytes());
    let const_result = if override_result == 0 {
        0x0012u16
    } else {
        0x1012u16
    };
    data.extend_from_slice(&const_result.to_le_bytes());
    data.extend_from_slice(&0x000fu16.to_le_bytes());

    let file_size = data.len() as u32;
    data[0..8].copy_from_slice(b"dex\n035\0");
    write_test_u32(&mut data, 32, file_size);
    write_test_u32(&mut data, 36, DEX_HEADER_SIZE_U32);
    write_test_u32(&mut data, 40, DEX_ENDIAN_CONSTANT);
    write_test_u32(&mut data, 56, 4);
    write_test_u32(&mut data, 60, string_ids_off as u32);
    write_test_u32(&mut data, 64, 3);
    write_test_u32(&mut data, 68, type_ids_off as u32);
    write_test_u32(&mut data, 72, 1);
    write_test_u32(&mut data, 76, proto_ids_off as u32);
    write_test_u32(&mut data, 88, 2);
    write_test_u32(&mut data, 92, method_ids_off as u32);
    write_test_u32(&mut data, 96, 1);
    write_test_u32(&mut data, 100, class_defs_off as u32);
    write_test_u32(&mut data, 104, file_size - data_off as u32);
    write_test_u32(&mut data, 108, data_off as u32);

    for (index, offset) in string_offsets.iter().enumerate() {
        write_test_u32(&mut data, string_ids_off + index * STRING_ID_SIZE, *offset);
    }

    write_test_u32(&mut data, type_ids_off, 0);
    write_test_u32(&mut data, type_ids_off + TYPE_ID_SIZE, 1);
    write_test_u32(&mut data, type_ids_off + TYPE_ID_SIZE * 2, 2);

    write_test_u32(&mut data, proto_ids_off, 2);
    write_test_u32(&mut data, proto_ids_off + 4, 2);

    write_test_u16(&mut data, method_ids_off, 0);
    write_test_u16(&mut data, method_ids_off + 2, 0);
    write_test_u32(&mut data, method_ids_off + 4, 3);
    write_test_u16(&mut data, method_ids_off + METHOD_ID_SIZE, 1);
    write_test_u16(&mut data, method_ids_off + METHOD_ID_SIZE + 2, 0);
    write_test_u32(&mut data, method_ids_off + METHOD_ID_SIZE + 4, 3);

    write_test_u32(&mut data, class_defs_off, 0);
    write_test_u32(
        &mut data,
        class_defs_off + 4,
        JAVA_ACC_PUBLIC | JAVA_ACC_FINAL,
    );
    write_test_u32(&mut data, class_defs_off + 8, 1);
    write_test_u32(&mut data, class_defs_off + 16, u32::MAX);
    write_test_u32(&mut data, class_defs_off + 24, class_data_off);

    data
}

// cite: frameworks-base ViewGroup.java#shouldDelayChildPressedState
// cite: frameworks-base FrameLayout.java#shouldDelayChildPressedState
// cite: frameworks-base LinearLayout.java#shouldDelayChildPressedState
// cite: frameworks-base RelativeLayout.java#shouldDelayChildPressedState
// cite: frameworks-base ScrollView.java#shouldDelayChildPressedState
// cite: frameworks-base HorizontalScrollView.java#shouldDelayChildPressedState
#[test]
fn frameworks_view_group_pressed_state_delay_uses_runtime_class_hierarchy() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/view/ViewGroup;".to_owned(),
        name: "shouldDelayChildPressedState".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };

    for class_descriptor in [
        "Landroid/view/ViewGroup;",
        "Landroid/widget/FrameLayout;",
        "Landroid/widget/LinearLayout;",
        "Landroid/widget/RelativeLayout;",
        "Landroid/widget/ScrollView;",
        "Landroid/widget/HorizontalScrollView;",
    ] {
        let mut declared_method = method.clone();
        declared_method.class_descriptor = class_descriptor.to_owned();
        assert!(view_group_should_delay_child_pressed_state_method(
            &declared_method
        ));
    }

    for (class_descriptor, expected) in [
        ("Landroid/view/ViewGroup;", 1),
        ("Landroid/widget/FrameLayout;", 0),
        ("Landroid/widget/LinearLayout;", 0),
        ("Landroid/widget/RelativeLayout;", 0),
        ("Landroid/widget/ScrollView;", 1),
        ("Landroid/widget/HorizontalScrollView;", 1),
        ("Lcom/android/internal/policy/DecorView;", 0),
        ("Landroid/widget/TableLayout;", 0),
        ("Landroid/widget/ListView;", 1),
    ] {
        let receiver = DexValue::Reference(std::rc::Rc::from(format!(
            "{class_descriptor}:pressed-state-test"
        )));
        let invocation = test_invocation(vec![receiver]);

        assert_eq!(
            program
                .invoke_builtin_method_from_class_hierarchy(class_descriptor, &method, &invocation,)
                .expect("framework hierarchy lookup should complete"),
            Some(DexExecutionResult::Int(expected)),
            "{class_descriptor}"
        );
    }
}

// cite: art interpreter_common.h#DoInvoke, FindMethodToCall
#[test]
fn art_virtual_view_group_override_precedes_framework_builtin() {
    for (platform_superclass, override_result) in [
        ("Landroid/view/ViewGroup;", 0),
        ("Landroid/widget/FrameLayout;", 1),
    ] {
        let dex_file = crate::dex::parse_dex_file(
            "view_group_pressed_state_override.dex",
            view_group_pressed_state_override_dex(platform_superclass, override_result),
        )
        .expect("view group override DEX should parse");
        let program = DexProgram::new(vec![dex_file]);
        let receiver = program.allocate_object("Lcom/example/DelayGroup;".to_owned());
        let mut handler = DexRunInvocationHandler {
            program: &program,
            current_dex_file_index: 0,
            current_class_descriptor: None,
            depth: 0,
        };
        let result = handler
            .invoke(DexInvocation {
                kind: crate::dex_interpreter::DexInvokeKind::Virtual,
                range: false,
                method_index: 1,
                argument_registers: Default::default(),
                arguments: smallvec::smallvec![receiver],
            })
            .expect("virtual ViewGroup override should execute");
        let DexInvocationResult::RawValue(result) = result else {
            panic!("virtual ViewGroup override should return a value, got {result:?}");
        };

        assert_eq!(
            result.into_value().expect("boolean result"),
            DexValue::Int(override_result),
            "{platform_superclass}"
        );
    }
}

#[test]
fn frameworks_display_real_metrics_fill_the_display_metrics_fields() {
    // cite: frameworks-base Display.java#getRealMetrics
    // cite: frameworks-base DisplayMetrics.java#setToDefaults
    let program =
        DexProgram::with_config(Vec::new(), WinedroidConfig::with_display_size(600, 1000));
    let get_real_metrics = builtin_method(
        "Landroid/view/Display;",
        "getRealMetrics",
        "V",
        &["Landroid/util/DisplayMetrics;"],
    );
    let display = DexValue::Reference(std::rc::Rc::from("Landroid/view/Display;.bootstrap"));
    let metrics = program.allocate_object("Landroid/util/DisplayMetrics;".to_owned());

    assert_eq!(
        program.invoke_builtin_method(
            &get_real_metrics,
            Some(&test_invocation(vec![display, metrics.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    for (name, expected) in [
        ("widthPixels", 600),
        ("heightPixels", 1000),
        ("densityDpi", BOOTSTRAP_DISPLAY_DENSITY_DPI),
        ("noncompatDensityDpi", BOOTSTRAP_DISPLAY_DENSITY_DPI),
    ] {
        assert_eq!(
            program.read_stored_instance_field(&metrics, &display_metrics_field(name, "I")),
            Some(DexValue::Int(expected))
        );
    }
    assert_eq!(
        program.read_stored_instance_field(&metrics, &display_metrics_field("density", "F")),
        Some(DexValue::Float(BOOTSTRAP_DISPLAY_DENSITY))
    );
    assert_eq!(
        program.read_stored_instance_field(&metrics, &display_metrics_field("xdpi", "F")),
        Some(DexValue::Float(BOOTSTRAP_DISPLAY_DENSITY_DPI as f32))
    );
}

// cite: frameworks-base FrameLayout.java#LayoutParams
// cite: frameworks-base ViewGroup.java#MarginLayoutParams
#[test]
fn builtin_frame_layout_params_store_size_and_are_margin_layout_params() {
    let program = DexProgram::new(Vec::new());
    let class = "Landroid/widget/FrameLayout$LayoutParams;";
    let init = builtin_method(class, "<init>", "V", &["I", "I"]);
    let init_gravity = builtin_method(class, "<init>", "V", &["I", "I", "I"]);
    let params = program.allocate_object(class.to_owned());
    let centered = program.allocate_object(class.to_owned());
    let field = |owner: &str, name: &str| DexFieldId {
        class_descriptor: owner.to_owned(),
        type_descriptor: "I".to_owned(),
        name: name.to_owned(),
    };

    assert_eq!(
        program.invoke_builtin_method(
            &init,
            Some(&test_invocation(vec![
                params.clone(),
                DexValue::Int(-1),
                DexValue::Int(-2)
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(
            &params,
            &field("Landroid/view/ViewGroup$LayoutParams;", "width")
        ),
        Some(DexValue::Int(-1))
    );
    program.invoke_builtin_method(
        &init_gravity,
        Some(&test_invocation(vec![
            centered.clone(),
            DexValue::Int(-1),
            DexValue::Int(-1),
            DexValue::Int(17),
        ])),
    );
    assert_eq!(
        program.read_stored_instance_field(&centered, &field(class, "gravity")),
        Some(DexValue::Int(17))
    );
    assert!(program.value_matches_type(&params, "Landroid/view/ViewGroup$MarginLayoutParams;"));
}

// cite: frameworks-base Scroller.java#startScroll, computeScrollOffset, fling, abortAnimation
#[test]
fn builtin_scroller_scrolls_over_time_and_flings_within_bounds() {
    let program = DexProgram::new(Vec::new());
    let class = "Landroid/widget/Scroller;";
    let init = builtin_method(class, "<init>", "V", &["Landroid/content/Context;"]);
    let start_scroll = builtin_method(class, "startScroll", "V", &["I", "I", "I", "I", "I"]);
    let compute = builtin_method(class, "computeScrollOffset", "Z", &[]);
    let curr_x = builtin_method(class, "getCurrX", "I", &[]);
    let final_x = builtin_method(class, "getFinalX", "I", &[]);
    let finished = builtin_method(class, "isFinished", "Z", &[]);
    let fling = builtin_method(
        class,
        "fling",
        "V",
        &["I", "I", "I", "I", "I", "I", "I", "I"],
    );
    let abort = builtin_method(class, "abortAnimation", "V", &[]);
    let scroller = program.allocate_object(class.to_owned());
    let context = DexValue::Reference(std::rc::Rc::from("Landroid/content/Context;.bootstrap"));
    let call = |method: &DexMethodId, arguments: &[i32]| {
        let mut values = vec![scroller.clone()];
        values.extend(arguments.iter().map(|value| DexValue::Int(*value)));
        program.invoke_builtin_method(method, Some(&test_invocation(values)))
    };
    let int = |result: Option<DexExecutionResult>| match result {
        Some(DexExecutionResult::Int(value)) => value,
        other => panic!("expected an int, got {other:?}"),
    };
    let set_time = |millis: i64| program.runtime.borrow_mut().main_thread_uptime_millis = millis;

    program.invoke_builtin_method(
        &init,
        Some(&test_invocation(vec![scroller.clone(), context])),
    );
    assert_eq!(int(call(&finished, &[])), 1);
    assert_eq!(int(call(&compute, &[])), 0);

    set_time(1000);
    call(&start_scroll, &[0, 0, 300, 0, 100]);
    set_time(1050);
    assert_eq!(int(call(&compute, &[])), 1);
    let midway = int(call(&curr_x, &[]));
    assert!(midway > 0 && midway < 300, "midway position {midway}");
    set_time(1100);
    assert_eq!(int(call(&compute, &[])), 1);
    assert_eq!(int(call(&curr_x, &[])), 300);
    assert_eq!(int(call(&finished, &[])), 1);

    call(&fling, &[0, 0, 4000, 0, 0, 500, 0, 0]);
    let fling_end = int(call(&final_x, &[]));
    assert!(fling_end > 0 && fling_end <= 500, "fling end {fling_end}");
    call(&abort, &[]);
    assert_eq!(int(call(&curr_x, &[])), fling_end);
}

// cite: frameworks-base ViewConfiguration.java#getScaledPagingTouchSlop, getScaledOverflingDistance, getLongPressTimeout, DEFAULT_LONG_PRESS_TIMEOUT
#[test]
fn builtin_view_configuration_reports_aosp_defaults() {
    let program = DexProgram::new(Vec::new());
    let configuration = DexValue::Reference(std::rc::Rc::from(
        "Landroid/view/ViewConfiguration;.bootstrap",
    ));
    let get = |name: &str| {
        program.invoke_builtin_method(
            &builtin_method("Landroid/view/ViewConfiguration;", name, "I", &[]),
            Some(&test_invocation(vec![configuration.clone()])),
        )
    };

    assert_eq!(
        get("getScaledPagingTouchSlop"),
        Some(DexExecutionResult::Int(16))
    );
    assert_eq!(
        get("getScaledOverflingDistance"),
        Some(DexExecutionResult::Int(6))
    );
    assert_eq!(
        get("getLongPressTimeout"),
        Some(DexExecutionResult::Int(400))
    );
    assert_eq!(get("getScaledUnknownThing"), None);
}

// cite: frameworks-base ViewStub.java#ViewStub, setLayoutResource, getLayoutResource, getInflatedId
#[test]
fn builtin_view_stub_starts_gone_and_stores_its_layout() {
    let program = DexProgram::new(Vec::new());
    let class = "Landroid/view/ViewStub;";
    let init = builtin_method(class, "<init>", "V", &["Landroid/content/Context;"]);
    let set_layout = builtin_method(class, "setLayoutResource", "V", &["I"]);
    let get_layout = builtin_method(class, "getLayoutResource", "I", &[]);
    let get_inflated = builtin_method(class, "getInflatedId", "I", &[]);
    let stub = program.allocate_object(class.to_owned());
    let context = DexValue::Reference(std::rc::Rc::from("Landroid/content/Context;.bootstrap"));
    let call = |method: &DexMethodId, mut arguments: Vec<DexValue>| {
        arguments.insert(0, stub.clone());
        program.invoke_builtin_method(method, Some(&test_invocation(arguments)))
    };

    assert_eq!(call(&init, vec![context]), Some(DexExecutionResult::Void));
    assert_eq!(
        program.read_stored_instance_field(&stub, &view_visibility_field()),
        Some(DexValue::Int(8))
    );
    assert_eq!(call(&get_layout, vec![]), Some(DexExecutionResult::Int(0)));
    assert_eq!(
        call(&get_inflated, vec![]),
        Some(DexExecutionResult::Int(-1))
    );
    call(&set_layout, vec![DexValue::Int(0x7f0d_0001)]);
    assert_eq!(
        call(&get_layout, vec![]),
        Some(DexExecutionResult::Int(0x7f0d_0001))
    );
    assert!(program.value_matches_type(&stub, "Landroid/view/View;"));
}

// cite: frameworks-base View.java#View, setFitsSystemWindows, getFitsSystemWindows
#[test]
fn builtin_view_fits_system_windows_comes_from_xml_and_setter() {
    let program = DexProgram::new(Vec::new());
    let get = builtin_method("Landroid/view/View;", "getFitsSystemWindows", "Z", &[]);
    let set = builtin_method("Landroid/view/View;", "setFitsSystemWindows", "V", &["Z"]);
    let init = builtin_method(
        "Landroid/widget/FrameLayout;",
        "<init>",
        "V",
        &["Landroid/content/Context;", "Landroid/util/AttributeSet;"],
    );
    let attributes = DexValue::Reference(std::rc::Rc::from("Landroid/util/AttributeSet;:test"));
    program.runtime.borrow_mut().xml_parsers.insert(
        map_identity_key(&attributes),
        XmlParserState {
            resource_id: 0,
            events: vec![crate::binary_xml::XmlEvent::Start(
                crate::binary_xml::XmlStartElement {
                    name: "FrameLayout".to_owned(),
                    namespace: None,
                    attributes: vec![crate::binary_xml::XmlAttribute {
                        name: "fitsSystemWindows".to_owned(),
                        namespace: Some("http://schemas.android.com/apk/res/android".to_owned()),
                        value: crate::binary_xml::XmlValue::Boolean(true),
                    }],
                },
            )],
            position: XmlParserPosition::Event(0),
            closed: false,
        },
    );
    let inflated = program.allocate_object("Landroid/widget/FrameLayout;".to_owned());
    let context = DexValue::Reference(std::rc::Rc::from("Landroid/content/Context;.bootstrap"));
    program.invoke_builtin_method(
        &init,
        Some(&test_invocation(vec![
            inflated.clone(),
            context,
            attributes,
        ])),
    );
    let plain = program.allocate_object("Landroid/view/View;".to_owned());
    let call = |method: &DexMethodId, view: &DexValue, mut arguments: Vec<DexValue>| {
        arguments.insert(0, view.clone());
        program.invoke_builtin_method(method, Some(&test_invocation(arguments)))
    };

    assert_eq!(
        call(&get, &inflated, vec![]),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(call(&get, &plain, vec![]), Some(DexExecutionResult::Int(0)));
    call(&set, &plain, vec![DexValue::Int(1)]);
    assert_eq!(call(&get, &plain, vec![]), Some(DexExecutionResult::Int(1)));
}
