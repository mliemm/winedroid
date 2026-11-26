use super::super::*;
use super::helpers::test_invocation;

#[test]
fn bootstraps_display_metrics_with_android_baseline_density() {
    let density = DexFieldId {
        class_descriptor: "Landroid/util/DisplayMetrics;".to_owned(),
        type_descriptor: "F".to_owned(),
        name: "density".to_owned(),
    };
    let scaled_density = DexFieldId {
        class_descriptor: "Landroid/util/DisplayMetrics;".to_owned(),
        type_descriptor: "F".to_owned(),
        name: "scaledDensity".to_owned(),
    };
    let xdpi = DexFieldId {
        class_descriptor: "Landroid/util/DisplayMetrics;".to_owned(),
        type_descriptor: "F".to_owned(),
        name: "xdpi".to_owned(),
    };
    let density_dpi = DexFieldId {
        class_descriptor: "Landroid/util/DisplayMetrics;".to_owned(),
        type_descriptor: "I".to_owned(),
        name: "densityDpi".to_owned(),
    };
    let width = DexFieldId {
        class_descriptor: "Landroid/util/DisplayMetrics;".to_owned(),
        type_descriptor: "I".to_owned(),
        name: "widthPixels".to_owned(),
    };
    let height = DexFieldId {
        class_descriptor: "Landroid/util/DisplayMetrics;".to_owned(),
        type_descriptor: "I".to_owned(),
        name: "heightPixels".to_owned(),
    };

    assert_eq!(
        bootstrap_instance_field(&density),
        Some(DexValue::Float(BOOTSTRAP_DISPLAY_DENSITY))
    );
    assert_eq!(
        bootstrap_instance_field(&scaled_density),
        Some(DexValue::Float(BOOTSTRAP_DISPLAY_DENSITY))
    );
    assert_eq!(
        bootstrap_instance_field(&xdpi),
        Some(DexValue::Float(BOOTSTRAP_DISPLAY_DENSITY_DPI as f32))
    );
    assert_eq!(
        bootstrap_instance_field(&density_dpi),
        Some(DexValue::Int(BOOTSTRAP_DISPLAY_DENSITY_DPI))
    );
    assert_eq!(
        bootstrap_instance_field(&width),
        Some(DexValue::Int(BOOTSTRAP_DISPLAY_WIDTH))
    );
    assert_eq!(
        bootstrap_instance_field(&height),
        Some(DexValue::Int(BOOTSTRAP_DISPLAY_HEIGHT))
    );
}

#[test]
fn bootstraps_configuration_with_android_baseline_font_scale() {
    let font_scale = DexFieldId {
        class_descriptor: "Landroid/content/res/Configuration;".to_owned(),
        type_descriptor: "F".to_owned(),
        name: "fontScale".to_owned(),
    };

    assert_eq!(
        bootstrap_instance_field(&font_scale),
        Some(DexValue::Float(BOOTSTRAP_FONT_SCALE))
    );
}

#[test]
fn bootstraps_priority_queue_collection_hierarchy() {
    assert_eq!(
        bootstrap_superclass_descriptor("Ljava/util/PriorityQueue;"),
        Some("Ljava/util/AbstractQueue;")
    );
    assert_eq!(
        bootstrap_superclass_descriptor("Ljava/util/AbstractQueue;"),
        Some("Ljava/util/AbstractCollection;")
    );
    assert_eq!(
        bootstrap_direct_interface_descriptors("Ljava/util/AbstractQueue;"),
        &["Ljava/util/Queue;"]
    );
    assert!(is_bootstrap_interface_descriptor("Ljava/util/Queue;"));
}

// cite: libcore LinkedBlockingQueue.java#LinkedBlockingQueue
#[test]
fn bootstraps_linked_blocking_queue_hierarchy() {
    assert_eq!(
        bootstrap_superclass_descriptor("Ljava/util/concurrent/LinkedBlockingQueue;"),
        Some("Ljava/util/AbstractQueue;")
    );
    assert_eq!(
        bootstrap_direct_interface_descriptors("Ljava/util/concurrent/LinkedBlockingQueue;"),
        &[
            "Ljava/util/concurrent/BlockingQueue;",
            "Ljava/io/Serializable;"
        ]
    );
    assert_eq!(
        bootstrap_direct_interface_descriptors("Ljava/util/concurrent/BlockingQueue;"),
        &["Ljava/util/Queue;"]
    );
    assert!(is_bootstrap_interface_descriptor(
        "Ljava/util/concurrent/BlockingQueue;"
    ));
}

// cite: frameworks-base Resources.java#NotFoundException
#[test]
fn bootstrap_resources_not_found_exception_extends_runtime_exception() {
    assert_eq!(
        bootstrap_superclass_descriptor("Landroid/content/res/Resources$NotFoundException;"),
        Some("Ljava/lang/RuntimeException;")
    );
}

#[test]
fn bootstraps_recording_canvas_as_canvas() {
    assert_eq!(
        bootstrap_superclass_descriptor("Landroid/graphics/RecordingCanvas;"),
        Some("Landroid/graphics/Canvas;")
    );
    assert_eq!(
        bootstrap_superclass_descriptor("Landroid/graphics/Canvas;"),
        Some("Ljava/lang/Object;")
    );
}

#[test]
fn bootstraps_android_widget_view_hierarchy() {
    assert_eq!(
        bootstrap_superclass_descriptor("Landroid/widget/FrameLayout;"),
        Some("Landroid/view/ViewGroup;")
    );
    assert_eq!(
        bootstrap_superclass_descriptor("Landroid/widget/LinearLayout;"),
        Some("Landroid/view/ViewGroup;")
    );
    assert_eq!(
        bootstrap_superclass_descriptor("Landroid/widget/Button;"),
        Some("Landroid/widget/TextView;")
    );
    assert_eq!(
        bootstrap_superclass_descriptor("Landroid/widget/ImageButton;"),
        Some("Landroid/widget/ImageView;")
    );
    assert_eq!(
        bootstrap_superclass_descriptor("Landroid/view/ViewGroup;"),
        Some("Landroid/view/View;")
    );
    assert_eq!(
        bootstrap_superclass_descriptor("Landroid/view/View;"),
        Some("Ljava/lang/Object;")
    );
}

// cite: frameworks-base AutofillManager.java#AutofillCallback
#[test]
fn bootstraps_autofill_callback_framework_metadata() {
    let descriptor = "Landroid/view/autofill/AutofillManager$AutofillCallback;";

    assert!(is_bootstrap_class_descriptor(descriptor));
    assert_eq!(
        bootstrap_superclass_descriptor(descriptor),
        Some("Ljava/lang/Object;")
    );
    assert_eq!(
        bootstrap_new_instance_kind(descriptor),
        Some(BootstrapNewInstanceKind::Abstract)
    );

    let program = DexProgram::new(Vec::new());
    let class = test_invocation(vec![DexValue::Reference(std::rc::Rc::from(format!(
        "Ljava/lang/Class;:{descriptor}"
    )))]);
    assert_eq!(
        program.class_get_modifiers(&class),
        Some((JAVA_ACC_PUBLIC | JAVA_ACC_ABSTRACT | DEX_ACC_STATIC) as i32)
    );
}

#[test]
fn bootstraps_saved_state_is_restored_as_false() {
    let field = DexFieldId {
        class_descriptor: "Landroidx/savedstate/internal/SavedStateRegistryImpl;".to_owned(),
        type_descriptor: "Z".to_owned(),
        name: "isRestored".to_owned(),
    };

    assert_eq!(bootstrap_instance_field(&field), Some(DexValue::Int(0)));
}

#[test]
fn bootstraps_saved_state_attached_as_false_and_accepts_writes() {
    let field = DexFieldId {
        class_descriptor: "Landroidx/savedstate/internal/SavedStateRegistryImpl;".to_owned(),
        type_descriptor: "Z".to_owned(),
        name: "attached".to_owned(),
    };

    assert_eq!(bootstrap_instance_field(&field), Some(DexValue::Int(0)));
    assert!(bootstrap_instance_field_write(&field));
}

#[test]
fn bootstraps_component_activity_context_aware_helper() {
    let field = DexFieldId {
        class_descriptor: "Landroidx/activity/ComponentActivity;".to_owned(),
        type_descriptor: "Landroidx/activity/contextaware/ContextAwareHelper;".to_owned(),
        name: "mContextAwareHelper".to_owned(),
    };

    assert_eq!(
        bootstrap_instance_field(&field),
        Some(DexValue::Reference(std::rc::Rc::from(
            field.display_signature()
        )))
    );
}

#[test]
fn bootstraps_component_activity_on_back_pressed_dispatcher() {
    let field = DexFieldId {
        class_descriptor: "Landroidx/activity/ComponentActivity;".to_owned(),
        type_descriptor: "Landroidx/activity/OnBackPressedDispatcher;".to_owned(),
        name: "mOnBackPressedDispatcher".to_owned(),
    };

    assert_eq!(
        bootstrap_instance_field(&field),
        Some(DexValue::Reference(std::rc::Rc::from(
            field.display_signature()
        )))
    );
}

#[test]
fn bootstraps_component_activity_content_layout_id_as_zero() {
    let field = DexFieldId {
        class_descriptor: "Landroidx/activity/ComponentActivity;".to_owned(),
        type_descriptor: "I".to_owned(),
        name: "mContentLayoutId".to_owned(),
    };

    assert_eq!(bootstrap_instance_field(&field), Some(DexValue::Int(0)));
}

#[test]
fn bootstraps_context_aware_helper_listeners() {
    let field = DexFieldId {
        class_descriptor: "Landroidx/activity/contextaware/ContextAwareHelper;".to_owned(),
        type_descriptor: "Ljava/util/Set;".to_owned(),
        name: "listeners".to_owned(),
    };

    assert_eq!(
        bootstrap_instance_field(&field),
        Some(DexValue::Reference(std::rc::Rc::from(
            field.display_signature()
        )))
    );
}

#[test]
fn bootstraps_work_data_values_as_empty_map_reference() {
    let field = DexFieldId {
        class_descriptor: "Landroidx/work/Data;".to_owned(),
        type_descriptor: "Ljava/util/Map;".to_owned(),
        name: "values".to_owned(),
    };

    assert_eq!(
        bootstrap_instance_field(&field),
        Some(DexValue::Reference(std::rc::Rc::from(
            field.display_signature()
        )))
    );
}

#[test]
fn bootstraps_observable_observers_as_empty_array_list_reference() {
    let field = DexFieldId {
        class_descriptor: "Landroid/database/Observable;".to_owned(),
        type_descriptor: "Ljava/util/ArrayList;".to_owned(),
        name: "mObservers".to_owned(),
    };

    assert_eq!(
        bootstrap_instance_field(&field),
        Some(DexValue::Reference(std::rc::Rc::from(
            field.display_signature()
        )))
    );
}

#[test]
fn bootstraps_fragment_activity_lifecycle_registry() {
    let field = DexFieldId {
        class_descriptor: "Landroidx/fragment/app/FragmentActivity;".to_owned(),
        type_descriptor: "Landroidx/lifecycle/LifecycleRegistry;".to_owned(),
        name: "mFragmentLifecycleRegistry".to_owned(),
    };

    assert_eq!(
        bootstrap_instance_field(&field),
        Some(DexValue::Reference(std::rc::Rc::from(
            field.display_signature()
        )))
    );
}

#[test]
fn bootstraps_fragment_activity_fragment_controller() {
    let field = DexFieldId {
        class_descriptor: "Landroidx/fragment/app/FragmentActivity;".to_owned(),
        type_descriptor: "Landroidx/fragment/app/FragmentController;".to_owned(),
        name: "mFragments".to_owned(),
    };

    assert_eq!(
        bootstrap_instance_field(&field),
        Some(DexValue::Reference(std::rc::Rc::from(
            field.display_signature()
        )))
    );
}

#[test]
fn bootstraps_appcompat_delegate_field() {
    let field = DexFieldId {
        class_descriptor: "Landroidx/appcompat/app/AppCompatActivity;".to_owned(),
        type_descriptor: "Landroidx/appcompat/app/AppCompatDelegate;".to_owned(),
        name: "mDelegate".to_owned(),
    };

    assert_eq!(
        bootstrap_instance_field(&field),
        Some(DexValue::Reference(std::rc::Rc::from(
            field.display_signature()
        )))
    );
}

#[test]
fn bootstraps_appcompat_resources_field() {
    let field = DexFieldId {
        class_descriptor: "Landroidx/appcompat/app/AppCompatActivity;".to_owned(),
        type_descriptor: "Landroid/content/res/Resources;".to_owned(),
        name: "mResources".to_owned(),
    };

    assert_eq!(
        bootstrap_instance_field(&field),
        Some(DexValue::Reference(std::rc::Rc::from(
            field.display_signature()
        )))
    );
}

#[test]
fn bootstraps_content_frame_layout_rect_field() {
    let field = DexFieldId {
        class_descriptor: "Landroidx/appcompat/widget/ContentFrameLayout;".to_owned(),
        type_descriptor: "Landroid/graphics/Rect;".to_owned(),
        name: "g".to_owned(),
    };

    assert_eq!(
        bootstrap_instance_field(&field),
        Some(DexValue::Reference(std::rc::Rc::from(
            field.display_signature()
        )))
    );

    for name in ["a", "b", "c", "d", "e", "f"] {
        let field = DexFieldId {
            class_descriptor: "Landroidx/appcompat/widget/ContentFrameLayout;".to_owned(),
            type_descriptor: "Landroid/util/TypedValue;".to_owned(),
            name: name.to_owned(),
        };

        assert_eq!(
            bootstrap_instance_field(&field),
            Some(DexValue::Reference(std::rc::Rc::from(
                field.display_signature()
            )))
        );
    }
}

#[test]
fn bootstraps_obfuscated_main_dispatcher_immediate_field() {
    let field = DexFieldId {
        class_descriptor: "LI1/c;".to_owned(),
        type_descriptor: "LI1/c;".to_owned(),
        name: "e".to_owned(),
    };

    assert_eq!(
        bootstrap_instance_field(&field),
        Some(DexValue::Reference(std::rc::Rc::from(
            field.display_signature()
        )))
    );
}

#[test]
fn bootstraps_view_pager_recycler_view_field() {
    let field = DexFieldId {
        class_descriptor: "Landroidx/viewpager2/widget/ViewPager2;".to_owned(),
        type_descriptor: "Landroidx/recyclerview/widget/RecyclerView;".to_owned(),
        name: "mRecyclerView".to_owned(),
    };

    assert_eq!(
        bootstrap_instance_field(&field),
        Some(DexValue::Reference(std::rc::Rc::from(
            field.display_signature()
        )))
    );
}

#[test]
fn bootstraps_view_pager_accessibility_provider_field() {
    let field = DexFieldId {
        class_descriptor: "Landroidx/viewpager2/widget/ViewPager2;".to_owned(),
        type_descriptor: "Landroidx/viewpager2/widget/ViewPager2$AccessibilityProvider;".to_owned(),
        name: "mAccessibilityProvider".to_owned(),
    };

    assert_eq!(
        bootstrap_instance_field(&field),
        Some(DexValue::Reference(std::rc::Rc::from(
            field.display_signature()
        )))
    );
}

#[test]
fn bootstraps_recycler_view_adapter_as_null() {
    let field = DexFieldId {
        class_descriptor: "Landroidx/recyclerview/widget/RecyclerView;".to_owned(),
        type_descriptor: "Landroidx/recyclerview/widget/RecyclerView$Adapter;".to_owned(),
        name: "mAdapter".to_owned(),
    };

    assert_eq!(bootstrap_instance_field(&field), Some(DexValue::Null));
}

#[test]
fn bootstraps_recycler_view_view_flinger() {
    let view_flinger = DexFieldId {
        class_descriptor: "Landroidx/recyclerview/widget/RecyclerView;".to_owned(),
        type_descriptor: "Lm0/f0;".to_owned(),
        name: "e0".to_owned(),
    };
    let recycler_view = DexFieldId {
        class_descriptor: "Lm0/f0;".to_owned(),
        type_descriptor: "Landroidx/recyclerview/widget/RecyclerView;".to_owned(),
        name: "g".to_owned(),
    };
    let scroller = DexFieldId {
        class_descriptor: "Lm0/f0;".to_owned(),
        type_descriptor: "Landroid/widget/OverScroller;".to_owned(),
        name: "c".to_owned(),
    };

    assert_eq!(
        bootstrap_instance_field(&view_flinger),
        Some(DexValue::Reference(std::rc::Rc::from(
            view_flinger.display_signature()
        )))
    );
    assert_eq!(
        bootstrap_instance_field(&recycler_view),
        Some(DexValue::Reference(std::rc::Rc::from(
            "Landroidx/recyclerview/widget/RecyclerView;.bootstrap"
        )))
    );
    assert_eq!(
        bootstrap_instance_field(&scroller),
        Some(DexValue::Reference(std::rc::Rc::from(
            scroller.display_signature()
        )))
    );
}

#[test]
fn bootstraps_recycler_view_child_helpers() {
    for (class_descriptor, type_descriptor, name, expected) in [
        (
            "Landroidx/recyclerview/widget/RecyclerView;",
            "Lm0/W;",
            "c",
            "Landroidx/recyclerview/widget/RecyclerView;.c:Lm0/W;",
        ),
        (
            "Landroidx/recyclerview/widget/RecyclerView;",
            "Lm0/b;",
            "e",
            "Landroidx/recyclerview/widget/RecyclerView;.e:Lm0/b;",
        ),
        (
            "Landroidx/recyclerview/widget/RecyclerView;",
            "LP/m;",
            "f",
            "Landroidx/recyclerview/widget/RecyclerView;.f:LP/m;",
        ),
        (
            "Landroidx/recyclerview/widget/RecyclerView;",
            "Lm0/c0;",
            "h0",
            "Landroidx/recyclerview/widget/RecyclerView;.h0:Lm0/c0;",
        ),
        (
            "Landroidx/recyclerview/widget/RecyclerView;",
            "Lm0/n;",
            "g0",
            "Landroidx/recyclerview/widget/RecyclerView;.g0:Lm0/n;",
        ),
        (
            "Lm0/W;",
            "Ljava/util/ArrayList;",
            "a",
            "Lm0/W;.a:Ljava/util/ArrayList;",
        ),
        (
            "Lm0/W;",
            "Ljava/util/ArrayList;",
            "c",
            "Lm0/W;.c:Ljava/util/ArrayList;",
        ),
        (
            "Lm0/b;",
            "Ljava/lang/Object;",
            "c",
            "Lm0/b;.c:Ljava/lang/Object;",
        ),
        (
            "Lm0/b;",
            "Ljava/lang/Object;",
            "d",
            "Lm0/b;.d:Ljava/lang/Object;",
        ),
        ("LP/m;", "Ljava/lang/Object;", "c", "Lm0/c;.bootstrap"),
    ] {
        let field = DexFieldId {
            class_descriptor: class_descriptor.to_owned(),
            type_descriptor: type_descriptor.to_owned(),
            name: name.to_owned(),
        };

        assert_eq!(
            bootstrap_instance_field(&field),
            Some(DexValue::Reference(std::rc::Rc::from(expected.to_owned())))
        );
    }
}

#[test]
fn scopes_bootstrap_field_references_to_their_owner() {
    let program = DexProgram::new(Vec::new());
    let recycler_view = view_reference(2131296519);
    let recycler_field = DexFieldId {
        class_descriptor: "Landroidx/recyclerview/widget/RecyclerView;".to_owned(),
        type_descriptor: "Lm0/W;".to_owned(),
        name: "c".to_owned(),
    };
    let owner_field = DexFieldId {
        class_descriptor: "Lm0/W;".to_owned(),
        type_descriptor: "Landroidx/recyclerview/widget/RecyclerView;".to_owned(),
        name: "h".to_owned(),
    };

    let recycler_child = program
        .read_bootstrap_instance_field(&recycler_view, &recycler_field)
        .expect("RecyclerView child helper should bootstrap");

    match &recycler_child {
        DexValue::Reference(reference) => {
            assert!(reference.starts_with("Lm0/W;:bootstrap-field:"));
        }
        value => panic!("expected scoped reference, got {value:?}"),
    }
    assert_eq!(
        program.read_bootstrap_instance_field(&recycler_child, &owner_field),
        Some(recycler_view)
    );
}

#[test]
fn bootstraps_view_pager_current_item_observer_field() {
    let field = DexFieldId {
        class_descriptor: "Landroidx/viewpager2/widget/ViewPager2;".to_owned(),
        type_descriptor: "Landroidx/recyclerview/widget/RecyclerView$AdapterDataObserver;"
            .to_owned(),
        name: "mCurrentItemDataSetChangeObserver".to_owned(),
    };

    assert_eq!(
        bootstrap_instance_field(&field),
        Some(DexValue::Reference(std::rc::Rc::from(
            field.display_signature()
        )))
    );
}

#[test]
fn bootstraps_view_pager_pending_adapter_state_as_null() {
    let field = DexFieldId {
        class_descriptor: "Landroidx/viewpager2/widget/ViewPager2;".to_owned(),
        type_descriptor: "Landroid/os/Parcelable;".to_owned(),
        name: "mPendingAdapterState".to_owned(),
    };

    assert_eq!(bootstrap_instance_field(&field), Some(DexValue::Null));
}

#[test]
fn bootstraps_view_pager_external_page_change_callbacks_field() {
    let field = DexFieldId {
        class_descriptor: "Landroidx/viewpager2/widget/ViewPager2;".to_owned(),
        type_descriptor: "Landroidx/viewpager2/widget/CompositeOnPageChangeCallback;".to_owned(),
        name: "mExternalPageChangeCallbacks".to_owned(),
    };

    assert_eq!(
        bootstrap_instance_field(&field),
        Some(DexValue::Reference(std::rc::Rc::from(
            field.display_signature()
        )))
    );
}

#[test]
fn bootstraps_composite_page_change_callbacks_list_field() {
    let field = DexFieldId {
        class_descriptor: "Landroidx/viewpager2/widget/CompositeOnPageChangeCallback;".to_owned(),
        type_descriptor: "Ljava/util/List;".to_owned(),
        name: "mCallbacks".to_owned(),
    };

    assert_eq!(
        bootstrap_instance_field(&field),
        Some(DexValue::Reference(std::rc::Rc::from(
            field.display_signature()
        )))
    );
}

#[test]
fn bootstraps_tab_layout_selected_listeners_field() {
    let field = DexFieldId {
        class_descriptor: "Lcom/google/android/material/tabs/TabLayout;".to_owned(),
        type_descriptor: "Ljava/util/ArrayList;".to_owned(),
        name: "selectedListeners".to_owned(),
    };

    assert_eq!(
        bootstrap_instance_field(&field),
        Some(DexValue::Reference(std::rc::Rc::from(
            field.display_signature()
        )))
    );
}

#[test]
fn bootstraps_lifecycle_registry_main_thread_enforcement_as_false() {
    let field = DexFieldId {
        class_descriptor: "Landroidx/lifecycle/LifecycleRegistry;".to_owned(),
        type_descriptor: "Z".to_owned(),
        name: "enforceMainThread".to_owned(),
    };

    assert_eq!(bootstrap_instance_field(&field), Some(DexValue::Int(0)));
}

#[test]
fn bootstraps_lifecycle_registry_state_as_initialized() {
    let field = DexFieldId {
        class_descriptor: "Landroidx/lifecycle/LifecycleRegistry;".to_owned(),
        type_descriptor: "Landroidx/lifecycle/Lifecycle$State;".to_owned(),
        name: "state".to_owned(),
    };

    assert_eq!(
        bootstrap_instance_field(&field),
        Some(DexValue::Reference(std::rc::Rc::from(
            "Landroidx/lifecycle/Lifecycle$State;.INITIALIZED"
        )))
    );
    assert!(bootstrap_instance_field_write(&field));
}

#[test]
fn bootstraps_obfuscated_lifecycle_registry_state_as_initialized() {
    let field = DexFieldId {
        class_descriptor: "Landroidx/lifecycle/u;".to_owned(),
        type_descriptor: "Landroidx/lifecycle/l;".to_owned(),
        name: "d".to_owned(),
    };

    assert_eq!(
        bootstrap_instance_field(&field),
        Some(DexValue::Reference(std::rc::Rc::from(
            "Landroidx/lifecycle/l;.b"
        )))
    );
}

#[test]
fn bootstraps_lifecycle_registry_weak_owner() {
    let field = DexFieldId {
        class_descriptor: "Landroidx/lifecycle/LifecycleRegistry;".to_owned(),
        type_descriptor: "Ljava/lang/ref/WeakReference;".to_owned(),
        name: "lifecycleOwner".to_owned(),
    };

    assert_eq!(
        bootstrap_instance_field(&field),
        Some(DexValue::Reference(std::rc::Rc::from(
            field.display_signature()
        )))
    );
}

#[test]
fn bootstraps_lifecycle_adapter_cache_maps() {
    for name in ["a", "b"] {
        let field = DexFieldId {
            class_descriptor: "Landroidx/lifecycle/c;".to_owned(),
            type_descriptor: "Ljava/util/HashMap;".to_owned(),
            name: name.to_owned(),
        };

        assert_eq!(
            bootstrap_instance_field(&field),
            Some(DexValue::Reference(std::rc::Rc::from(
                field.display_signature()
            )))
        );
    }
}

#[test]
fn bootstraps_safe_iterable_map_ends_as_null() {
    for name in ["a", "b"] {
        let field = DexFieldId {
            class_descriptor: "Lp/f;".to_owned(),
            type_descriptor: "Lp/c;".to_owned(),
            name: name.to_owned(),
        };

        assert_eq!(bootstrap_instance_field(&field), Some(DexValue::Null));
    }
}

#[test]
fn bootstraps_safe_iterable_map_node_links_as_null() {
    for name in ["c", "d"] {
        let field = DexFieldId {
            class_descriptor: "Lp/c;".to_owned(),
            type_descriptor: "Lp/c;".to_owned(),
            name: name.to_owned(),
        };

        assert_eq!(bootstrap_instance_field(&field), Some(DexValue::Null));
    }
}

#[test]
fn bootstraps_back_dispatcher_owner_as_null() {
    let field = DexFieldId {
        class_descriptor: "Lb/a;".to_owned(),
        type_descriptor: "La/m;".to_owned(),
        name: "b".to_owned(),
    };

    assert_eq!(bootstrap_instance_field(&field), Some(DexValue::Null));
}

#[test]
fn bootstraps_fragment_manager_host_as_unattached() {
    let field = DexFieldId {
        class_descriptor: "Ld0/F;".to_owned(),
        type_descriptor: "Ld0/s;".to_owned(),
        name: "t".to_owned(),
    };

    assert_eq!(bootstrap_instance_field(&field), Some(DexValue::Null));
}

#[test]
fn bootstraps_fragment_manager_pending_actions_as_empty_list() {
    for name in ["d", "I", "J"] {
        let field = DexFieldId {
            class_descriptor: "Ld0/F;".to_owned(),
            type_descriptor: "Ljava/util/ArrayList;".to_owned(),
            name: name.to_owned(),
        };

        assert_eq!(
            bootstrap_instance_field(&field),
            Some(DexValue::Reference(std::rc::Rc::from(
                field.display_signature()
            )))
        );
    }
}

#[test]
fn bootstraps_obfuscated_auto_night_secondary_handle_as_empty() {
    let field = DexFieldId {
        class_descriptor: "Ld0/g;".to_owned(),
        type_descriptor: "Ljava/lang/Object;".to_owned(),
        name: "b".to_owned(),
    };

    assert_eq!(bootstrap_instance_field(&field), Some(DexValue::Null));
}

#[test]
fn bootstraps_component_activity_view_model_store_as_empty() {
    let field = DexFieldId {
        class_descriptor: "La/m;".to_owned(),
        type_descriptor: "Landroidx/lifecycle/K;".to_owned(),
        name: "f".to_owned(),
    };

    assert_eq!(bootstrap_instance_field(&field), Some(DexValue::Null));
}

#[test]
fn bootstraps_creation_extras_backing_map() {
    let field = DexFieldId {
        class_descriptor: "Lg0/b;".to_owned(),
        type_descriptor: "Ljava/util/LinkedHashMap;".to_owned(),
        name: "a".to_owned(),
    };

    assert_eq!(
        bootstrap_instance_field(&field),
        Some(DexValue::Reference(std::rc::Rc::from(
            field.display_signature()
        )))
    );
}

#[test]
fn bootstraps_appcompat_delegate_cache_as_empty() {
    let field = DexFieldId {
        class_descriptor: "Lg/m;".to_owned(),
        type_descriptor: "Lg/F;".to_owned(),
        name: "w".to_owned(),
    };

    assert_eq!(bootstrap_instance_field(&field), Some(DexValue::Null));
}

#[test]
fn bootstraps_appcompat_delegate_action_bar_as_empty() {
    for (name, type_descriptor) in [
        ("o", "Lg/P;"),
        ("M", "[Lg/E;"),
        ("X", "Lg/B;"),
        ("Y", "Lg/B;"),
        ("l", "Landroid/view/Window;"),
        ("S", "Landroid/content/res/Configuration;"),
    ] {
        let field = DexFieldId {
            class_descriptor: "Lg/F;".to_owned(),
            type_descriptor: type_descriptor.to_owned(),
            name: name.to_owned(),
        };

        assert_eq!(bootstrap_instance_field(&field), Some(DexValue::Null));
    }
}

#[test]
fn bootstraps_appcompat_delegate_context_field() {
    for (name, type_descriptor, value) in [
        (
            "k",
            "Landroid/content/Context;",
            "Landroid/content/Context;.bootstrap",
        ),
        ("m", "Lg/A;", "Lg/F;.m:Lg/A;"),
    ] {
        let field = DexFieldId {
            class_descriptor: "Lg/F;".to_owned(),
            type_descriptor: type_descriptor.to_owned(),
            name: name.to_owned(),
        };

        assert_eq!(
            bootstrap_instance_field(&field),
            Some(DexValue::Reference(std::rc::Rc::from(value.to_owned())))
        );
    }
}

#[test]
fn bootstraps_target_app_atomic_boolean_field() {
    let field = DexFieldId {
        class_descriptor: "Lt2/g;".to_owned(),
        type_descriptor: "Ljava/util/concurrent/atomic/AtomicBoolean;".to_owned(),
        name: "f".to_owned(),
    };

    assert_eq!(
        bootstrap_instance_field(&field),
        Some(DexValue::Reference(std::rc::Rc::from(
            field.display_signature()
        )))
    );
}

#[test]
fn bootstraps_target_app_component_container_field() {
    let field = DexFieldId {
        class_descriptor: "Lt2/g;".to_owned(),
        type_descriptor: "Ly2/f;".to_owned(),
        name: "d".to_owned(),
    };

    assert_eq!(
        bootstrap_instance_field(&field),
        Some(DexValue::Reference(std::rc::Rc::from(
            field.display_signature()
        )))
    );
}

#[test]
fn bootstraps_component_container_hash_map_field() {
    let field = DexFieldId {
        class_descriptor: "Ly2/f;".to_owned(),
        type_descriptor: "Ljava/util/HashMap;".to_owned(),
        name: "b".to_owned(),
    };

    assert_eq!(
        bootstrap_instance_field(&field),
        Some(DexValue::Reference(std::rc::Rc::from(
            field.display_signature()
        )))
    );
}

#[test]
fn bootstraps_crashlytics_component_dependency_field() {
    let field = DexFieldId {
        class_descriptor: "La3/e;".to_owned(),
        type_descriptor: "Le3/r;".to_owned(),
        name: "a".to_owned(),
    };

    assert_eq!(
        bootstrap_instance_field(&field),
        Some(DexValue::Reference(std::rc::Rc::from(
            field.display_signature()
        )))
    );
}

#[test]
fn bootstraps_crashlytics_settings_dependency_field() {
    let field = DexFieldId {
        class_descriptor: "Le3/r;".to_owned(),
        type_descriptor: "Le3/u;".to_owned(),
        name: "b".to_owned(),
    };

    assert_eq!(
        bootstrap_instance_field(&field),
        Some(DexValue::Reference(std::rc::Rc::from(
            field.display_signature()
        )))
    );
}

#[test]
fn bootstraps_crashlytics_settings_preferences_field() {
    let field = DexFieldId {
        class_descriptor: "Le3/u;".to_owned(),
        type_descriptor: "Landroid/content/SharedPreferences;".to_owned(),
        name: "a".to_owned(),
    };

    assert_eq!(
        bootstrap_instance_field(&field),
        Some(DexValue::Reference(std::rc::Rc::from(
            field.display_signature()
        )))
    );
}

#[test]
fn bootstraps_crashlytics_settings_lock_field() {
    let field = DexFieldId {
        class_descriptor: "Le3/u;".to_owned(),
        type_descriptor: "Ljava/lang/Object;".to_owned(),
        name: "c".to_owned(),
    };

    assert_eq!(
        bootstrap_instance_field(&field),
        Some(DexValue::Reference(std::rc::Rc::from(
            field.display_signature()
        )))
    );
}

#[test]
fn bootstraps_firebase_analytics_backend_field() {
    let field = DexFieldId {
        class_descriptor: "Lcom/google/firebase/analytics/FirebaseAnalytics;".to_owned(),
        type_descriptor: "Lcom/google/android/gms/internal/measurement/j7;".to_owned(),
        name: "a".to_owned(),
    };

    assert_eq!(
        bootstrap_instance_field(&field),
        Some(DexValue::Reference(std::rc::Rc::from(
            field.display_signature()
        )))
    );
}

#[test]
fn bootstraps_package_info_application_info_field() {
    let program = DexProgram::new(Vec::new());
    let application_info = DexFieldId {
        class_descriptor: "Landroid/content/pm/PackageInfo;".to_owned(),
        type_descriptor: "Landroid/content/pm/ApplicationInfo;".to_owned(),
        name: "applicationInfo".to_owned(),
    };
    let target_sdk = DexFieldId {
        class_descriptor: "Landroid/content/pm/ApplicationInfo;".to_owned(),
        type_descriptor: "I".to_owned(),
        name: "targetSdkVersion".to_owned(),
    };
    let parent_activity_name = DexFieldId {
        class_descriptor: "Landroid/content/pm/ActivityInfo;".to_owned(),
        type_descriptor: "Ljava/lang/String;".to_owned(),
        name: "parentActivityName".to_owned(),
    };
    let metadata = DexFieldId {
        class_descriptor: "Landroid/content/pm/ActivityInfo;".to_owned(),
        type_descriptor: "Landroid/os/Bundle;".to_owned(),
        name: "metaData".to_owned(),
    };

    assert_eq!(
        bootstrap_instance_field(&application_info),
        Some(DexValue::Reference(std::rc::Rc::from(
            application_info.display_signature()
        )))
    );
    assert_eq!(
        program.read_bootstrap_instance_field(
            &DexValue::Reference(std::rc::Rc::from(
                "Landroid/content/pm/ApplicationInfo;.bootstrap",
            )),
            &target_sdk,
        ),
        Some(DexValue::Int(BOOTSTRAP_ANDROID_SDK_INT))
    );
    assert_eq!(
        bootstrap_instance_field(&parent_activity_name),
        Some(DexValue::Null)
    );
    assert_eq!(bootstrap_instance_field(&metadata), Some(DexValue::Null));
}

#[test]
fn application_info_target_sdk_version_uses_manifest_runtime_value() {
    // cite: frameworks-base ApplicationInfo.java#targetSdkVersion
    let mut program = DexProgram::new(Vec::new());
    program.target_sdk_version = 28;
    let field = DexFieldId {
        class_descriptor: "Landroid/content/pm/ApplicationInfo;".to_owned(),
        type_descriptor: "I".to_owned(),
        name: "targetSdkVersion".to_owned(),
    };

    assert_eq!(
        program.read_bootstrap_instance_field(
            &DexValue::Reference(std::rc::Rc::from(
                "Landroid/content/pm/ApplicationInfo;.bootstrap",
            )),
            &field,
        ),
        Some(DexValue::Int(28))
    );
}

#[test]
fn allows_saved_state_restored_state_bootstrap_write() {
    let field = DexFieldId {
        class_descriptor: "Landroidx/savedstate/internal/SavedStateRegistryImpl;".to_owned(),
        type_descriptor: "Landroid/os/Bundle;".to_owned(),
        name: "restoredState".to_owned(),
    };

    assert!(bootstrap_instance_field_write(&field));
}

#[test]
fn bootstraps_context_aware_helper_context_as_null_and_accepts_writes() {
    let field = DexFieldId {
        class_descriptor: "Landroidx/activity/contextaware/ContextAwareHelper;".to_owned(),
        type_descriptor: "Landroid/content/Context;".to_owned(),
        name: "context".to_owned(),
    };

    assert_eq!(bootstrap_instance_field(&field), Some(DexValue::Null));
    assert!(bootstrap_instance_field_write(&field));
}
