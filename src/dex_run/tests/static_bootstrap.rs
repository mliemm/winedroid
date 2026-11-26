use super::super::*;
use super::helpers::*;

#[test]
fn builtin_ui_mode_manager_returns_normal_mode() {
    let program = DexProgram::new(Vec::new());
    for name in ["getCurrentModeType", "getNightMode"] {
        let method = DexMethodId {
            class_descriptor: "Landroid/app/UiModeManager;".to_owned(),
            name: name.to_owned(),
            proto: crate::dex::DexProto {
                shorty: "I".to_owned(),
                return_type: "I".to_owned(),
                parameters: Vec::new(),
            },
        };

        assert_eq!(
            program.invoke_builtin_method(&method, None),
            Some(DexExecutionResult::Int(1))
        );
    }
}

#[test]
fn builtin_app_ops_permission_to_op_returns_null() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/app/AppOpsManager;".to_owned(),
        name: "permissionToOp".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned()],
        },
    };
    let invocation = test_invocation(vec![DexValue::String(std::rc::Rc::from(
        "android.permission.ACCESS_NETWORK_STATE",
    ))]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
}

#[test]
fn builtin_location_manager_reports_providers_disabled() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/location/LocationManager;".to_owned(),
        name: "isProviderEnabled".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned()],
        },
    };
    let invocation = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from(
            "Landroid/location/LocationManager;.bootstrap",
        )),
        DexValue::String(std::rc::Rc::from("network")),
    ]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Int(0))
    );
}

#[test]
fn builtin_fragment_manager_find_fragment_by_tag_returns_null() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/app/FragmentManager;".to_owned(),
        name: "findFragmentByTag".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Landroid/app/Fragment;".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned()],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&method, None),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
}

#[test]
fn builtin_fragment_transaction_flow_returns_expected_values() {
    let program = DexProgram::new(Vec::new());
    let begin = DexMethodId {
        class_descriptor: "Landroid/app/FragmentManager;".to_owned(),
        name: "beginTransaction".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/app/FragmentTransaction;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let add = DexMethodId {
        class_descriptor: "Landroid/app/FragmentTransaction;".to_owned(),
        name: "add".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LLL".to_owned(),
            return_type: "Landroid/app/FragmentTransaction;".to_owned(),
            parameters: vec![
                "Landroid/app/Fragment;".to_owned(),
                "Ljava/lang/String;".to_owned(),
            ],
        },
    };
    let commit = DexMethodId {
        class_descriptor: "Landroid/app/FragmentTransaction;".to_owned(),
        name: "commit".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let execute = DexMethodId {
        class_descriptor: "Landroid/app/FragmentManager;".to_owned(),
        name: "executePendingTransactions".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&begin, None),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Landroid/app/FragmentTransaction;")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&add, None),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Landroid/app/FragmentTransaction;")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&commit, None),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&execute, None),
        Some(DexExecutionResult::Int(1))
    );
}

#[test]
fn builtin_fragment_constructor_returns_void() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/app/Fragment;".to_owned(),
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

// cite: libcore Collections.java#EMPTY_MAP, EMPTY_LIST, EMPTY_SET, emptyMap, emptyList, emptySet
#[test]
fn collections_empty_fields_match_factory_values() {
    let program = DexProgram::new(Vec::new());
    for (name, descriptor, factory) in [
        ("EMPTY_MAP", "Ljava/util/Map;", "emptyMap"),
        ("EMPTY_LIST", "Ljava/util/List;", "emptyList"),
        ("EMPTY_SET", "Ljava/util/Set;", "emptySet"),
    ] {
        let field = DexFieldId {
            class_descriptor: "Ljava/util/Collections;".to_owned(),
            type_descriptor: descriptor.to_owned(),
            name: name.to_owned(),
        };
        let method = builtin_method("Ljava/util/Collections;", factory, descriptor, &[]);
        assert_eq!(
            program.invoke_builtin_method(&method, None),
            bootstrap_static_field(&field).map(DexExecutionResult::Value)
        );
    }
}

#[test]
fn bootstraps_report_fragment_companion() {
    let field = DexFieldId {
        class_descriptor: "Landroidx/lifecycle/ReportFragment;".to_owned(),
        type_descriptor: "Landroidx/lifecycle/ReportFragment$Companion;".to_owned(),
        name: "Companion".to_owned(),
    };

    assert_eq!(
        bootstrap_static_field(&field),
        Some(DexValue::Reference(std::rc::Rc::from(
            field.display_signature()
        )))
    );
}

#[test]
fn bootstraps_report_fragment_lifecycle_callbacks_companion() {
    let field = DexFieldId {
        class_descriptor: "Landroidx/lifecycle/ReportFragment$LifecycleCallbacks;".to_owned(),
        type_descriptor: "Landroidx/lifecycle/ReportFragment$LifecycleCallbacks$Companion;"
            .to_owned(),
        name: "Companion".to_owned(),
    };

    assert_eq!(
        bootstrap_static_field(&field),
        Some(DexValue::Reference(std::rc::Rc::from(
            field.display_signature()
        )))
    );
}

#[test]
fn bootstraps_android_sdk_int() {
    let field = DexFieldId {
        class_descriptor: "Landroid/os/Build$VERSION;".to_owned(),
        type_descriptor: "I".to_owned(),
        name: "SDK_INT".to_owned(),
    };

    assert_eq!(
        bootstrap_static_field(&field),
        Some(DexValue::Int(BOOTSTRAP_ANDROID_SDK_INT))
    );
}

#[test]
fn bootstraps_locale_constants() {
    let field = DexFieldId {
        class_descriptor: "Ljava/util/Locale;".to_owned(),
        type_descriptor: "Ljava/util/Locale;".to_owned(),
        name: "ROOT".to_owned(),
    };

    assert_eq!(
        bootstrap_static_field(&field),
        Some(DexValue::Reference(std::rc::Rc::from(
            "Ljava/util/Locale;:ROOT"
        )))
    );
}

#[test]
fn bootstraps_android_path_fill_type_static_fields() {
    let field = DexFieldId {
        class_descriptor: "Landroid/graphics/Path$FillType;".to_owned(),
        type_descriptor: "Landroid/graphics/Path$FillType;".to_owned(),
        name: "WINDING".to_owned(),
    };

    assert_eq!(
        bootstrap_static_field(&field),
        Some(DexValue::Reference(std::rc::Rc::from(
            "Landroid/graphics/Path$FillType;.WINDING"
        )))
    );
}

#[test]
fn bootstraps_android_paint_style_static_fields() {
    let field = DexFieldId {
        class_descriptor: "Landroid/graphics/Paint$Style;".to_owned(),
        type_descriptor: "Landroid/graphics/Paint$Style;".to_owned(),
        name: "STROKE".to_owned(),
    };

    assert_eq!(
        bootstrap_static_field(&field),
        Some(DexValue::Reference(std::rc::Rc::from(
            "Landroid/graphics/Paint$Style;.STROKE"
        )))
    );
}

#[test]
fn bootstraps_android_image_view_scale_type_static_fields() {
    // cite: frameworks-base ImageView.java#ScaleType
    let field = DexFieldId {
        class_descriptor: "Landroid/widget/ImageView$ScaleType;".to_owned(),
        type_descriptor: "Landroid/widget/ImageView$ScaleType;".to_owned(),
        name: "FIT_CENTER".to_owned(),
    };

    assert_eq!(
        bootstrap_static_field(&field),
        Some(DexValue::Reference(std::rc::Rc::from(
            "Landroid/widget/ImageView$ScaleType;.FIT_CENTER"
        )))
    );
}

#[test]
fn bootstraps_android_bitmap_config_static_fields() {
    let field = DexFieldId {
        class_descriptor: "Landroid/graphics/Bitmap$Config;".to_owned(),
        type_descriptor: "Landroid/graphics/Bitmap$Config;".to_owned(),
        name: "ALPHA_8".to_owned(),
    };

    assert_eq!(
        bootstrap_static_field(&field),
        Some(DexValue::Reference(std::rc::Rc::from(
            "Landroid/graphics/Bitmap$Config;.ALPHA_8"
        )))
    );
}

#[test]
fn bootstraps_android_layout_alignment_static_fields() {
    let field = DexFieldId {
        class_descriptor: "Landroid/text/Layout$Alignment;".to_owned(),
        type_descriptor: "Landroid/text/Layout$Alignment;".to_owned(),
        name: "ALIGN_NORMAL".to_owned(),
    };

    assert_eq!(
        bootstrap_static_field(&field),
        Some(DexValue::Reference(std::rc::Rc::from(
            "Landroid/text/Layout$Alignment;.ALIGN_NORMAL"
        )))
    );
}

#[test]
fn bootstraps_android_text_direction_heuristics_static_fields() {
    let field = DexFieldId {
        class_descriptor: "Landroid/text/TextDirectionHeuristics;".to_owned(),
        type_descriptor: "Landroid/text/TextDirectionHeuristic;".to_owned(),
        name: "FIRSTSTRONG_LTR".to_owned(),
    };

    assert_eq!(
        bootstrap_static_field(&field),
        Some(DexValue::Reference(std::rc::Rc::from(
            "Landroid/text/TextDirectionHeuristics;.FIRSTSTRONG_LTR"
        )))
    );
}

#[test]
fn bootstraps_android_text_utils_truncate_at_static_fields() {
    let field = DexFieldId {
        class_descriptor: "Landroid/text/TextUtils$TruncateAt;".to_owned(),
        type_descriptor: "Landroid/text/TextUtils$TruncateAt;".to_owned(),
        name: "END".to_owned(),
    };

    assert_eq!(
        bootstrap_static_field(&field),
        Some(DexValue::Reference(std::rc::Rc::from(
            "Landroid/text/TextUtils$TruncateAt;.END"
        )))
    );
}

#[test]
fn bootstraps_android_path_direction_static_fields() {
    let field = DexFieldId {
        class_descriptor: "Landroid/graphics/Path$Direction;".to_owned(),
        type_descriptor: "Landroid/graphics/Path$Direction;".to_owned(),
        name: "CW".to_owned(),
    };

    assert_eq!(
        bootstrap_static_field(&field),
        Some(DexValue::Reference(std::rc::Rc::from(
            "Landroid/graphics/Path$Direction;.CW"
        )))
    );
}

#[test]
fn bootstraps_android_path_op_static_fields() {
    let field = DexFieldId {
        class_descriptor: "Landroid/graphics/Path$Op;".to_owned(),
        type_descriptor: "Landroid/graphics/Path$Op;".to_owned(),
        name: "INTERSECT".to_owned(),
    };

    assert_eq!(
        bootstrap_static_field(&field),
        Some(DexValue::Reference(std::rc::Rc::from(
            "Landroid/graphics/Path$Op;.INTERSECT"
        )))
    );
}

#[test]
fn bootstraps_android_blend_mode_static_fields() {
    let field = DexFieldId {
        class_descriptor: "Landroid/graphics/BlendMode;".to_owned(),
        type_descriptor: "Landroid/graphics/BlendMode;".to_owned(),
        name: "SRC_IN".to_owned(),
    };

    assert_eq!(
        bootstrap_static_field(&field),
        Some(DexValue::Reference(std::rc::Rc::from(
            "Landroid/graphics/BlendMode;.SRC_IN"
        )))
    );
}

#[test]
fn bootstraps_android_region_op_static_fields() {
    let field = DexFieldId {
        class_descriptor: "Landroid/graphics/Region$Op;".to_owned(),
        type_descriptor: "Landroid/graphics/Region$Op;".to_owned(),
        name: "INTERSECT".to_owned(),
    };

    assert_eq!(
        bootstrap_static_field(&field),
        Some(DexValue::Reference(std::rc::Rc::from(
            "Landroid/graphics/Region$Op;.INTERSECT"
        )))
    );
}

#[test]
fn bootstraps_android_color_space_named_static_fields() {
    let field = DexFieldId {
        class_descriptor: "Landroid/graphics/ColorSpace$Named;".to_owned(),
        type_descriptor: "Landroid/graphics/ColorSpace$Named;".to_owned(),
        name: "SRGB".to_owned(),
    };

    assert_eq!(
        bootstrap_static_field(&field),
        Some(DexValue::Reference(std::rc::Rc::from(
            "Landroid/graphics/ColorSpace$Named;.SRGB"
        )))
    );
}

#[test]
fn bootstraps_lifecycle_event_on_create() {
    let field = DexFieldId {
        class_descriptor: "Landroidx/lifecycle/Lifecycle$Event;".to_owned(),
        type_descriptor: "Landroidx/lifecycle/Lifecycle$Event;".to_owned(),
        name: "ON_CREATE".to_owned(),
    };

    assert_eq!(
        bootstrap_static_field(&field),
        Some(DexValue::Reference(std::rc::Rc::from(
            "Landroidx/lifecycle/Lifecycle$Event;.ON_CREATE"
        )))
    );
}

#[test]
fn bootstraps_obfuscated_lifecycle_event_static_fields() {
    for event in [
        "ON_ANY",
        "ON_CREATE",
        "ON_DESTROY",
        "ON_PAUSE",
        "ON_RESUME",
        "ON_START",
        "ON_STOP",
    ] {
        let field = DexFieldId {
            class_descriptor: "Landroidx/lifecycle/k;".to_owned(),
            type_descriptor: "Landroidx/lifecycle/k;".to_owned(),
            name: event.to_owned(),
        };

        assert_eq!(
            bootstrap_static_field(&field),
            Some(DexValue::Reference(std::rc::Rc::from(format!(
                "Landroidx/lifecycle/k;.{event}"
            ))))
        );
    }

    let companion = DexFieldId {
        class_descriptor: "Landroidx/lifecycle/k;".to_owned(),
        type_descriptor: "Landroidx/lifecycle/i;".to_owned(),
        name: "Companion".to_owned(),
    };

    assert_eq!(
        bootstrap_static_field(&companion),
        Some(DexValue::Reference(std::rc::Rc::from(
            companion.display_signature()
        )))
    );
}

#[test]
fn bootstraps_obfuscated_lifecycle_activity_callbacks_singleton() {
    for (class_descriptor, name, type_descriptor) in [
        ("Landroidx/lifecycle/E;", "a", "LC/b;"),
        (
            "Landroidx/lifecycle/E$a;",
            "Companion",
            "Landroidx/lifecycle/D;",
        ),
    ] {
        let field = DexFieldId {
            class_descriptor: class_descriptor.to_owned(),
            type_descriptor: type_descriptor.to_owned(),
            name: name.to_owned(),
        };

        assert_eq!(
            bootstrap_static_field(&field),
            Some(DexValue::Reference(std::rc::Rc::from(
                field.display_signature()
            )))
        );
    }
}

#[test]
fn bootstraps_fragment_manager_view_model_factory_singleton() {
    let field = DexFieldId {
        class_descriptor: "Ld0/H;".to_owned(),
        type_descriptor: "LW0/f;".to_owned(),
        name: "h".to_owned(),
    };

    assert_eq!(
        bootstrap_static_field(&field),
        Some(DexValue::Reference(std::rc::Rc::from("LW0/f;:Ld0/H.h")))
    );
}

#[test]
fn bootstraps_default_creation_extras_singleton() {
    let field = DexFieldId {
        class_descriptor: "Lg0/a;".to_owned(),
        type_descriptor: "Lg0/a;".to_owned(),
        name: "b".to_owned(),
    };

    assert_eq!(
        bootstrap_static_field(&field),
        Some(DexValue::Reference(std::rc::Rc::from(
            field.display_signature()
        )))
    );
}

#[test]
fn bootstraps_kotlin_reflection_factory_singleton() {
    let field = DexFieldId {
        class_descriptor: "Lz1/k;".to_owned(),
        type_descriptor: "Lz1/l;".to_owned(),
        name: "a".to_owned(),
    };

    assert_eq!(
        bootstrap_static_field(&field),
        Some(DexValue::Reference(std::rc::Rc::from(
            field.display_signature()
        )))
    );
}

#[test]
fn bootstraps_kotlin_class_primitive_wrapper_map() {
    let field = DexFieldId {
        class_descriptor: "Lz1/d;".to_owned(),
        type_descriptor: "Ljava/util/Map;".to_owned(),
        name: "b".to_owned(),
    };

    assert_eq!(
        bootstrap_static_field(&field),
        Some(DexValue::Reference(std::rc::Rc::from(
            field.display_signature()
        )))
    );
}

#[test]
fn bootstraps_appcompat_delegate_static_singletons() {
    for (name, type_descriptor) in [("g", "Ls/e;"), ("h", "Ljava/lang/Object;")] {
        let field = DexFieldId {
            class_descriptor: "Lg/t;".to_owned(),
            type_descriptor: type_descriptor.to_owned(),
            name: name.to_owned(),
        };

        assert_eq!(
            bootstrap_static_field(&field),
            Some(DexValue::Reference(std::rc::Rc::from(
                field.display_signature()
            )))
        );
    }
}

#[test]
fn bootstraps_target_calculator_static_singleton() {
    for (name, type_descriptor) in [("a", "Le1/e;"), ("b", "Lx0/b;")] {
        let field = DexFieldId {
            class_descriptor: "Lx0/b;".to_owned(),
            type_descriptor: type_descriptor.to_owned(),
            name: name.to_owned(),
        };

        assert_eq!(
            bootstrap_static_field(&field),
            Some(DexValue::Reference(std::rc::Rc::from(
                field.display_signature()
            )))
        );
    }
}

#[test]
fn bootstraps_appcompat_styleable_array() {
    let field = DexFieldId {
        class_descriptor: "Lf/a;".to_owned(),
        type_descriptor: "[I".to_owned(),
        name: "j".to_owned(),
    };

    assert_eq!(
        bootstrap_static_field(&field),
        Some(DexValue::Reference(std::rc::Rc::from(
            field.display_signature()
        )))
    );
}

#[test]
fn bootstraps_kotlin_random_provider_singletons() {
    for (name, type_descriptor, expected) in [
        ("a", "LB1/d;", "LB1/d;:LB1/e.a"),
        ("b", "LB1/a;", "LB1/a;:LB1/e.b"),
    ] {
        let field = DexFieldId {
            class_descriptor: "LB1/e;".to_owned(),
            type_descriptor: type_descriptor.to_owned(),
            name: name.to_owned(),
        };

        assert_eq!(
            bootstrap_static_field(&field),
            Some(DexValue::Reference(std::rc::Rc::from(expected.to_owned())))
        );
    }
}

#[test]
fn bootstraps_obfuscated_coroutine_dispatcher_singletons() {
    for (class_descriptor, name, type_descriptor) in [
        ("LH1/u;", "a", "LN1/d;"),
        ("LL1/l;", "a", "LI1/c;"),
        ("Lr1/j;", "a", "Lr1/j;"),
        ("Lr1/b;", "d", "Lr1/b;"),
        ("Lr1/e;", "a", "Lr1/e;"),
        ("LH1/l;", "b", "LH1/l;"),
        ("LH1/X;", "a", "LH1/X;"),
        ("Lp1/f;", "c", "Lp1/f;"),
        ("Ls1/a;", "a", "Ls1/a;"),
        ("Lq1/n;", "a", "Lq1/n;"),
    ] {
        let field = DexFieldId {
            class_descriptor: class_descriptor.to_owned(),
            type_descriptor: type_descriptor.to_owned(),
            name: name.to_owned(),
        };

        assert_eq!(
            bootstrap_static_field(&field),
            Some(DexValue::Reference(std::rc::Rc::from(
                field.display_signature()
            )))
        );
    }
}

#[test]
fn bootstraps_obfuscated_coroutine_function_singletons() {
    for (class_descriptor, type_descriptor, name) in [
        ("LH1/h;", "LH1/h;", "c"),
        ("LH1/h;", "LH1/h;", "d"),
        ("LL1/s;", "LL1/s;", "c"),
        ("LL1/s;", "LL1/s;", "d"),
        ("LL1/s;", "LL1/s;", "e"),
    ] {
        let field = DexFieldId {
            class_descriptor: class_descriptor.to_owned(),
            type_descriptor: type_descriptor.to_owned(),
            name: name.to_owned(),
        };

        assert_eq!(
            bootstrap_static_field(&field),
            Some(DexValue::Reference(std::rc::Rc::from(
                field.display_signature()
            )))
        );
    }
}

#[test]
fn bootstraps_obfuscated_coroutine_function_branch_fields() {
    let h_field = DexFieldId {
        class_descriptor: "LH1/h;".to_owned(),
        type_descriptor: "I".to_owned(),
        name: "b".to_owned(),
    };
    let s_field = DexFieldId {
        class_descriptor: "LL1/s;".to_owned(),
        type_descriptor: "I".to_owned(),
        name: "b".to_owned(),
    };

    assert_eq!(
        bootstrap_instance_field_for_object(
            &DexValue::Reference(std::rc::Rc::from("LH1/h;.c:LH1/h;")),
            &h_field
        ),
        Some(DexValue::Int(0))
    );
    assert_eq!(
        bootstrap_instance_field_for_object(
            &DexValue::Reference(std::rc::Rc::from("LH1/h;.d:LH1/h;")),
            &h_field
        ),
        Some(DexValue::Int(1))
    );
    assert_eq!(
        bootstrap_instance_field_for_object(
            &DexValue::Reference(std::rc::Rc::from("LL1/s;.c:LL1/s;")),
            &s_field
        ),
        Some(DexValue::Int(0))
    );
    assert_eq!(
        bootstrap_instance_field_for_object(
            &DexValue::Reference(std::rc::Rc::from("LL1/s;.d:LL1/s;")),
            &s_field
        ),
        Some(DexValue::Int(1))
    );
    assert_eq!(
        bootstrap_instance_field_for_object(
            &DexValue::Reference(std::rc::Rc::from("LL1/s;.e:LL1/s;")),
            &s_field
        ),
        Some(DexValue::Int(2))
    );
}

#[test]
fn bootstraps_lifecycle_state_static_fields() {
    for state in ["DESTROYED", "INITIALIZED", "CREATED", "STARTED", "RESUMED"] {
        let field = DexFieldId {
            class_descriptor: "Landroidx/lifecycle/Lifecycle$State;".to_owned(),
            type_descriptor: "Landroidx/lifecycle/Lifecycle$State;".to_owned(),
            name: state.to_owned(),
        };

        assert_eq!(
            bootstrap_static_field(&field),
            Some(DexValue::Reference(std::rc::Rc::from(format!(
                "Landroidx/lifecycle/Lifecycle$State;.{state}"
            ))))
        );
    }
}

#[test]
fn bootstraps_obfuscated_lifecycle_state_static_fields() {
    for name in ["a", "b", "c", "d", "e"] {
        let field = DexFieldId {
            class_descriptor: "Landroidx/lifecycle/l;".to_owned(),
            type_descriptor: "Landroidx/lifecycle/l;".to_owned(),
            name: name.to_owned(),
        };

        assert_eq!(
            bootstrap_static_field(&field),
            Some(DexValue::Reference(std::rc::Rc::from(
                field.display_signature()
            )))
        );
    }
}

#[test]
fn maps_obfuscated_lifecycle_state_ordinals() {
    for (name, ordinal) in [("a", 0), ("b", 1), ("c", 2), ("d", 3), ("e", 4)] {
        assert_eq!(
            lifecycle_state_ordinal(&DexValue::Reference(std::rc::Rc::from(format!(
                "Landroidx/lifecycle/l;.{name}:Landroidx/lifecycle/l;"
            )))),
            Some(ordinal)
        );
    }
}

#[test]
fn maps_lifecycle_event_ordinals() {
    for (name, ordinal) in [
        ("ON_CREATE", 0),
        ("ON_START", 1),
        ("ON_RESUME", 2),
        ("ON_PAUSE", 3),
        ("ON_STOP", 4),
        ("ON_DESTROY", 5),
        ("ON_ANY", 6),
    ] {
        assert_eq!(
            lifecycle_event_ordinal(&DexValue::Reference(std::rc::Rc::from(format!(
                "Landroidx/lifecycle/k;.{name}:Landroidx/lifecycle/k;"
            )))),
            Some(ordinal)
        );
    }
}

#[test]
fn builtin_enum_constructor_stores_name_and_ordinal() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Ljava/lang/Enum;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VLI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned(), "I".to_owned()],
        },
    };
    let ordinal = DexMethodId {
        class_descriptor: "Ljava/lang/Enum;".to_owned(),
        name: "ordinal".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let compare_to = DexMethodId {
        class_descriptor: "Ljava/lang/Enum;".to_owned(),
        name: "compareTo".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "IL".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["Ljava/lang/Enum;".to_owned()],
        },
    };
    let compare_to_object = DexMethodId {
        class_descriptor: "Ljava/lang/Enum;".to_owned(),
        name: "compareTo".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "IL".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let name = DexMethodId {
        class_descriptor: "Ljava/lang/Enum;".to_owned(),
        name: "name".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let value_of = DexMethodId {
        class_descriptor: "Ljava/lang/Enum;".to_owned(),
        name: "valueOf".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LLL".to_owned(),
            return_type: "Ljava/lang/Enum;".to_owned(),
            parameters: vec![
                "Ljava/lang/Class;".to_owned(),
                "Ljava/lang/String;".to_owned(),
            ],
        },
    };
    let left = DexValue::Object(DexObjectRef {
        id: 1,
        class_descriptor: std::rc::Rc::from("Lexample/State;"),
    });
    let right = DexValue::Object(DexObjectRef {
        id: 2,
        class_descriptor: std::rc::Rc::from("Lexample/State;"),
    });

    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![
                left.clone(),
                DexValue::String(std::rc::Rc::from("LEFT")),
                DexValue::Int(1)
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![
                right.clone(),
                DexValue::String(std::rc::Rc::from("RIGHT")),
                DexValue::Int(3)
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&ordinal, Some(&test_invocation(vec![right.clone()]))),
        Some(DexExecutionResult::Int(3))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &compare_to,
            Some(&test_invocation(vec![left.clone(), right.clone()]))
        ),
        Some(DexExecutionResult::Int(-2))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &compare_to_object,
            Some(&test_invocation(vec![right.clone(), left.clone()]))
        ),
        Some(DexExecutionResult::Int(2))
    );
    assert_eq!(
        program.invoke_builtin_method(&name, Some(&test_invocation(vec![left]))),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("LEFT")
        )))
    );

    let left_field_value = DexValue::Object(DexObjectRef {
        id: 1,
        class_descriptor: std::rc::Rc::from("Lexample/State;"),
    });
    assert_eq!(
        program.invoke_builtin_method(
            &value_of,
            Some(&test_invocation(vec![
                DexValue::Reference(std::rc::Rc::from("Ljava/lang/Class;:Lexample/State;")),
                DexValue::String(std::rc::Rc::from("LEFT")),
            ]))
        ),
        Some(DexExecutionResult::Value(left_field_value))
    );

    let right_field = DexFieldId {
        class_descriptor: "Lexample/State;".to_owned(),
        type_descriptor: "Lexample/State;".to_owned(),
        name: "RIGHT".to_owned(),
    };
    program.write_stored_static_field(&right_field, right_field_value());
    assert_eq!(
        program.invoke_builtin_method(
            &value_of,
            Some(&test_invocation(vec![
                DexValue::Reference(std::rc::Rc::from("Ljava/lang/Class;:Lexample/State;")),
                DexValue::String(std::rc::Rc::from("RIGHT")),
            ]))
        ),
        Some(DexExecutionResult::Value(right_field_value()))
    );

    let Some(DexExecutionResult::Throw(exception)) = program.invoke_builtin_method(
        &value_of,
        Some(&test_invocation(vec![
            DexValue::Reference(std::rc::Rc::from("Ljava/lang/Class;:Lexample/State;")),
            DexValue::String(std::rc::Rc::from("MISSING")),
        ])),
    ) else {
        panic!("Enum.valueOf should throw for a missing enum constant");
    };
    let message_field = DexFieldId {
        class_descriptor: "Ljava/lang/Throwable;".to_owned(),
        type_descriptor: "Ljava/lang/String;".to_owned(),
        name: "detailMessage".to_owned(),
    };
    assert!(program.value_matches_type(&exception, "Ljava/lang/IllegalArgumentException;"));
    assert!(program.value_matches_type(&exception, "Ljava/lang/RuntimeException;"));
    assert_eq!(
        program.read_stored_instance_field(&exception, &message_field),
        Some(DexValue::String(std::rc::Rc::from(
            "No enum constant Lexample/State;.MISSING"
        )))
    );
}

fn right_field_value() -> DexValue {
    DexValue::Reference(std::rc::Rc::from("Lexample/State;.RIGHT"))
}

#[test]
fn bootstraps_reached_enum_static_fields() {
    for unit in [
        "NANOSECONDS",
        "MICROSECONDS",
        "MILLISECONDS",
        "SECONDS",
        "MINUTES",
        "HOURS",
        "DAYS",
    ] {
        let field = DexFieldId {
            class_descriptor: "Ljava/util/concurrent/TimeUnit;".to_owned(),
            type_descriptor: "Ljava/util/concurrent/TimeUnit;".to_owned(),
            name: unit.to_owned(),
        };

        assert_eq!(
            bootstrap_static_field(&field),
            Some(DexValue::Reference(std::rc::Rc::from(format!(
                "Ljava/util/concurrent/TimeUnit;.{unit}"
            ))))
        );
    }

    let backoff_policy = DexFieldId {
        class_descriptor: "Landroidx/work/BackoffPolicy;".to_owned(),
        type_descriptor: "Landroidx/work/BackoffPolicy;".to_owned(),
        name: "LINEAR".to_owned(),
    };
    let network_type = DexFieldId {
        class_descriptor: "Landroidx/work/NetworkType;".to_owned(),
        type_descriptor: "Landroidx/work/NetworkType;".to_owned(),
        name: "CONNECTED".to_owned(),
    };
    let existing_work_policy = DexFieldId {
        class_descriptor: "Landroidx/work/ExistingWorkPolicy;".to_owned(),
        type_descriptor: "Landroidx/work/ExistingWorkPolicy;".to_owned(),
        name: "KEEP".to_owned(),
    };

    assert_eq!(
        bootstrap_static_field(&backoff_policy),
        Some(DexValue::Reference(std::rc::Rc::from(
            "Landroidx/work/BackoffPolicy;.LINEAR"
        )))
    );
    assert_eq!(
        bootstrap_static_field(&network_type),
        Some(DexValue::Reference(std::rc::Rc::from(
            "Landroidx/work/NetworkType;.CONNECTED"
        )))
    );
    assert_eq!(
        bootstrap_static_field(&existing_work_policy),
        Some(DexValue::Reference(std::rc::Rc::from(
            "Landroidx/work/ExistingWorkPolicy;.KEEP"
        )))
    );
}

#[test]
fn libcore_enum_equals_is_reference_identity() {
    // cite: libcore Enum.java#equals
    let program = DexProgram::new(Vec::new());
    let equals = DexMethodId {
        class_descriptor: "Ljava/lang/Enum;".to_owned(),
        name: "equals".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let left = DexValue::Object(DexObjectRef {
        id: 1,
        class_descriptor: std::rc::Rc::from("Lexample/State;"),
    });
    let right = DexValue::Object(DexObjectRef {
        id: 2,
        class_descriptor: std::rc::Rc::from("Lexample/State;"),
    });

    assert_eq!(
        program.invoke_builtin_method(
            &equals,
            Some(&test_invocation(vec![left.clone(), left.clone()]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&equals, Some(&test_invocation(vec![left, right]))),
        Some(DexExecutionResult::Int(0))
    );
}

#[test]
// cite: frameworks-base Build.java#SUPPORTED_ABIS, getStringList
fn frameworks_build_supported_abis_splits_the_device_abi_list_once() {
    let program = DexProgram::new(Vec::new());
    let field = DexFieldId {
        class_descriptor: "Landroid/os/Build;".to_owned(),
        type_descriptor: "[Ljava/lang/String;".to_owned(),
        name: "SUPPORTED_ABIS".to_owned(),
    };

    program
        .ensure_class_initialized_synchronously("Landroid/os/Build;", 0)
        .unwrap();
    let abis = program.read_stored_static_field(&field).unwrap();
    program
        .ensure_class_initialized_synchronously("Landroid/os/Build;", 0)
        .unwrap();

    assert_eq!(program.read_stored_static_field(&field), Some(abis.clone()));
    assert_eq!(program.array_length(&abis), Some(1));
    assert_eq!(
        program.read_array_value(&abis, 0),
        Some(DexValue::String(std::rc::Rc::from("arm64-v8a")))
    );
}
