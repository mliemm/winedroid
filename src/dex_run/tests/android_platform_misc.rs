use super::super::*;
use super::helpers::*;

#[test]
fn builtin_android_log_is_loggable_returns_false() {
    let program = DexProgram::new(Vec::new());
    let is_loggable = DexMethodId {
        class_descriptor: "Landroid/util/Log;".to_owned(),
        name: "isLoggable".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZLI".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned(), "I".to_owned()],
        },
    };
    let info = DexMethodId {
        class_descriptor: "Landroid/util/Log;".to_owned(),
        name: "i".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ILL".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec![
                "Ljava/lang/String;".to_owned(),
                "Ljava/lang/String;".to_owned(),
            ],
        },
    };
    let error = DexMethodId {
        class_descriptor: "Landroid/util/Log;".to_owned(),
        name: "e".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ILLL".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec![
                "Ljava/lang/String;".to_owned(),
                "Ljava/lang/String;".to_owned(),
                "Ljava/lang/Throwable;".to_owned(),
            ],
        },
    };
    let invocation = test_invocation(vec![
        DexValue::String(std::rc::Rc::from("AppCompatDelegate")),
        DexValue::String(std::rc::Rc::from("message")),
    ]);
    let error_invocation = test_invocation(vec![
        DexValue::String(std::rc::Rc::from("AppCompatDelegate")),
        DexValue::String(std::rc::Rc::from("message")),
        DexValue::Object(DexObjectRef {
            id: 1,
            class_descriptor: std::rc::Rc::from("Ljava/lang/Throwable;"),
        }),
    ]);

    assert_eq!(
        program.invoke_builtin_method(&is_loggable, None),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&info, Some(&invocation)),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&error, Some(&error_invocation)),
        Some(DexExecutionResult::Int(0))
    );
}

#[test]
fn builtin_android_trace_methods_are_noops() {
    let program = DexProgram::new(Vec::new());
    let begin_section = DexMethodId {
        class_descriptor: "Landroid/os/Trace;".to_owned(),
        name: "beginSection".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned()],
        },
    };
    let end_section = DexMethodId {
        class_descriptor: "Landroid/os/Trace;".to_owned(),
        name: "endSection".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let is_enabled = DexMethodId {
        class_descriptor: "Landroid/os/Trace;".to_owned(),
        name: "isEnabled".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };

    assert_eq!(
        program.invoke_builtin_method(
            &begin_section,
            Some(&test_invocation(vec![DexValue::String(std::rc::Rc::from(
                "Compose:initializeView"
            ))]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&end_section, None),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&is_enabled, None),
        Some(DexExecutionResult::Int(0))
    );
}

#[test]
fn builtin_android_strict_mode_vm_policy_is_available() {
    let program = DexProgram::new(Vec::new());
    let get_vm_policy = DexMethodId {
        class_descriptor: "Landroid/os/StrictMode;".to_owned(),
        name: "getVmPolicy".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/os/StrictMode$VmPolicy;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set_vm_policy = DexMethodId {
        class_descriptor: "Landroid/os/StrictMode;".to_owned(),
        name: "setVmPolicy".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/os/StrictMode$VmPolicy;".to_owned()],
        },
    };
    let policy = DexValue::Reference(std::rc::Rc::from(
        "Landroid/os/StrictMode$VmPolicy;:default",
    ));

    assert_eq!(
        program.invoke_builtin_method(&get_vm_policy, None),
        Some(DexExecutionResult::Value(policy.clone()))
    );
    assert_eq!(
        program.invoke_builtin_method(&set_vm_policy, Some(&test_invocation(vec![policy]))),
        Some(DexExecutionResult::Void)
    );
}

#[test]
fn builtin_content_observer_constructor_stores_handler() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Landroid/database/ContentObserver;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/os/Handler;".to_owned()],
        },
    };
    let observer = DexValue::Object(DexObjectRef {
        id: 12,
        class_descriptor: std::rc::Rc::from("Ls83;"),
    });
    let handler = DexValue::Object(DexObjectRef {
        id: 13,
        class_descriptor: std::rc::Rc::from("Landroid/os/Handler;"),
    });
    let handler_field = DexFieldId {
        class_descriptor: "Landroid/database/ContentObserver;".to_owned(),
        type_descriptor: "Landroid/os/Handler;".to_owned(),
        name: "mHandler".to_owned(),
    };

    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![observer.clone(), handler.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&observer, &handler_field),
        Some(handler)
    );
}

#[test]
fn builtin_animation_utils_current_time_uses_bootstrap_clock() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/view/animation/AnimationUtils;".to_owned(),
        name: "currentAnimationTimeMillis".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "J".to_owned(),
            return_type: "J".to_owned(),
            parameters: Vec::new(),
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&method, None),
        Some(DexExecutionResult::Value(DexValue::Long(0)))
    );
}

#[test]
fn builtin_android_vibrator_service_methods_are_noops() {
    let program = DexProgram::new(Vec::new());
    let get_default_vibrator = DexMethodId {
        class_descriptor: "Landroid/os/VibratorManager;".to_owned(),
        name: "getDefaultVibrator".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/os/Vibrator;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let has_vibrator = DexMethodId {
        class_descriptor: "Landroid/os/Vibrator;".to_owned(),
        name: "hasVibrator".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let vibrate = DexMethodId {
        class_descriptor: "Landroid/os/Vibrator;".to_owned(),
        name: "vibrate".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VJ".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["J".to_owned()],
        },
    };
    let manager = DexValue::Reference(std::rc::Rc::from(
        "Landroid/os/SystemVibratorManager;:vibrator_manager",
    ));
    let vibrator = DexValue::Reference(std::rc::Rc::from("Landroid/os/Vibrator;:default"));

    assert_eq!(
        program.invoke_builtin_method(&get_default_vibrator, Some(&test_invocation(vec![manager]))),
        Some(DexExecutionResult::Value(vibrator.clone()))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &has_vibrator,
            Some(&test_invocation(vec![vibrator.clone()]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &vibrate,
            Some(&test_invocation(vec![vibrator, DexValue::Long(20)]))
        ),
        Some(DexExecutionResult::Void)
    );
}

#[test]
fn builtin_sound_pool_builder_and_playback_methods_are_noop_audio_backend() {
    let program = DexProgram::new(Vec::new());
    let builder_constructor = DexMethodId {
        class_descriptor: "Landroid/media/SoundPool$Builder;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set_max_streams = DexMethodId {
        class_descriptor: "Landroid/media/SoundPool$Builder;".to_owned(),
        name: "setMaxStreams".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LI".to_owned(),
            return_type: "Landroid/media/SoundPool$Builder;".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let build = DexMethodId {
        class_descriptor: "Landroid/media/SoundPool$Builder;".to_owned(),
        name: "build".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/media/SoundPool;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let load = DexMethodId {
        class_descriptor: "Landroid/media/SoundPool;".to_owned(),
        name: "load".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LII".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec![
                "Landroid/content/Context;".to_owned(),
                "I".to_owned(),
                "I".to_owned(),
            ],
        },
    };
    let play = DexMethodId {
        class_descriptor: "Landroid/media/SoundPool;".to_owned(),
        name: "play".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "IIFFIIF".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec![
                "I".to_owned(),
                "F".to_owned(),
                "F".to_owned(),
                "I".to_owned(),
                "I".to_owned(),
                "F".to_owned(),
            ],
        },
    };
    let builder = DexValue::Object(DexObjectRef {
        id: 11,
        class_descriptor: std::rc::Rc::from("Landroid/media/SoundPool$Builder;"),
    });
    let sound_pool = DexValue::Reference(std::rc::Rc::from("Landroid/media/SoundPool;:default"));

    assert_eq!(
        program.invoke_builtin_method(
            &builder_constructor,
            Some(&test_invocation(vec![builder.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_max_streams,
            Some(&test_invocation(vec![builder.clone(), DexValue::Int(4)]))
        ),
        Some(DexExecutionResult::Value(builder.clone()))
    );
    assert_eq!(
        program.invoke_builtin_method(&build, Some(&test_invocation(vec![builder]))),
        Some(DexExecutionResult::Value(sound_pool.clone()))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &load,
            Some(&test_invocation(vec![
                sound_pool.clone(),
                DexValue::Reference(std::rc::Rc::from("Landroid/content/Context;.bootstrap")),
                DexValue::Int(2131820544),
                DexValue::Int(1),
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &play,
            Some(&test_invocation(vec![
                sound_pool,
                DexValue::Int(1),
                DexValue::Float(1.0),
                DexValue::Float(1.0),
                DexValue::Int(1),
                DexValue::Int(0),
                DexValue::Float(1.0),
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
}

#[test]
fn builtin_accessibility_manager_reports_disabled_by_default() {
    let program = DexProgram::new(Vec::new());
    let is_enabled = DexMethodId {
        class_descriptor: "Landroid/view/accessibility/AccessibilityManager;".to_owned(),
        name: "isEnabled".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let is_touch_exploration_enabled = DexMethodId {
        class_descriptor: "Landroid/view/accessibility/AccessibilityManager;".to_owned(),
        name: "isTouchExplorationEnabled".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let manager = DexValue::Reference(std::rc::Rc::from(
        "Landroid/view/accessibility/AccessibilityManager;:accessibility",
    ));

    assert_eq!(
        program.invoke_builtin_method(&is_enabled, Some(&test_invocation(vec![manager.clone()]))),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &is_touch_exploration_enabled,
            Some(&test_invocation(vec![manager]))
        ),
        Some(DexExecutionResult::Int(0))
    );
}

#[test]
fn builtin_accessibility_manager_tracks_state_listeners() {
    let program = DexProgram::new(Vec::new());
    let add_state = DexMethodId {
        class_descriptor: "Landroid/view/accessibility/AccessibilityManager;".to_owned(),
        name: "addAccessibilityStateChangeListener".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec![
                "Landroid/view/accessibility/AccessibilityManager$AccessibilityStateChangeListener;"
                    .to_owned(),
            ],
        },
    };
    let remove_state = DexMethodId {
        class_descriptor: "Landroid/view/accessibility/AccessibilityManager;".to_owned(),
        name: "removeAccessibilityStateChangeListener".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec![
                "Landroid/view/accessibility/AccessibilityManager$AccessibilityStateChangeListener;"
                    .to_owned(),
            ],
        },
    };
    let add_touch_exploration = DexMethodId {
        class_descriptor: "Landroid/view/accessibility/AccessibilityManager;".to_owned(),
        name: "addTouchExplorationStateChangeListener".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec![
                "Landroid/view/accessibility/AccessibilityManager$TouchExplorationStateChangeListener;"
                    .to_owned(),
            ],
        },
    };
    let remove_touch_exploration = DexMethodId {
        class_descriptor: "Landroid/view/accessibility/AccessibilityManager;".to_owned(),
        name: "removeTouchExplorationStateChangeListener".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec![
                "Landroid/view/accessibility/AccessibilityManager$TouchExplorationStateChangeListener;"
                    .to_owned(),
            ],
        },
    };
    let manager = DexValue::Reference(std::rc::Rc::from(
        "Landroid/view/accessibility/AccessibilityManager;:accessibility",
    ));
    let state_listener = DexValue::Reference(
        std::rc::Rc::from("Landroid/view/accessibility/AccessibilityManager$AccessibilityStateChangeListener;:compose"
            .to_owned()),
    );
    let touch_exploration_listener = DexValue::Reference(
        std::rc::Rc::from("Landroid/view/accessibility/AccessibilityManager$TouchExplorationStateChangeListener;:compose"
            .to_owned()),
    );
    let state_invocation = test_invocation(vec![manager.clone(), state_listener.clone()]);
    let touch_invocation =
        test_invocation(vec![manager.clone(), touch_exploration_listener.clone()]);

    assert_eq!(
        program.invoke_builtin_method(&add_state, Some(&state_invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&add_state, Some(&state_invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .accessibility_state_change_listeners
            .get(&map_identity_key(&manager))
            .cloned(),
        Some(vec![state_listener])
    );
    assert_eq!(
        program.invoke_builtin_method(&add_touch_exploration, Some(&touch_invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .accessibility_touch_exploration_state_change_listeners
            .get(&map_identity_key(&manager))
            .cloned(),
        Some(vec![touch_exploration_listener])
    );
    assert_eq!(
        program.invoke_builtin_method(&remove_state, Some(&state_invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&remove_state, Some(&state_invocation)),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&remove_touch_exploration, Some(&touch_invocation)),
        Some(DexExecutionResult::Int(1))
    );
}

#[test]
fn builtin_sparse_array_stores_values_by_int_key() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Landroid/util/SparseArray;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let put = DexMethodId {
        class_descriptor: "Landroid/util/SparseArray;".to_owned(),
        name: "put".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VIL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned(), "Ljava/lang/Object;".to_owned()],
        },
    };
    let size = DexMethodId {
        class_descriptor: "Landroid/util/SparseArray;".to_owned(),
        name: "size".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get = DexMethodId {
        class_descriptor: "Landroid/util/SparseArray;".to_owned(),
        name: "get".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LI".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let value_at = DexMethodId {
        class_descriptor: "Landroid/util/SparseArray;".to_owned(),
        name: "valueAt".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LI".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let key_at = DexMethodId {
        class_descriptor: "Landroid/util/SparseArray;".to_owned(),
        name: "keyAt".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "II".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let remove_at = DexMethodId {
        class_descriptor: "Landroid/util/SparseArray;".to_owned(),
        name: "removeAt".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let array = DexValue::Reference(std::rc::Rc::from("sparse-array"));
    let value = DexValue::Reference(std::rc::Rc::from("stored-value"));

    assert_eq!(
        program.invoke_builtin_method(&constructor, Some(&test_invocation(vec![array.clone()]))),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &put,
            Some(&test_invocation(vec![
                array.clone(),
                DexValue::Int(42),
                value.clone(),
            ])),
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&size, Some(&test_invocation(vec![array.clone()]))),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get,
            Some(&test_invocation(vec![array.clone(), DexValue::Int(42)])),
        ),
        Some(DexExecutionResult::Value(value.clone()))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &key_at,
            Some(&test_invocation(vec![array.clone(), DexValue::Int(0)])),
        ),
        Some(DexExecutionResult::Int(42))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &value_at,
            Some(&test_invocation(vec![array.clone(), DexValue::Int(0)])),
        ),
        Some(DexExecutionResult::Value(value))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &remove_at,
            Some(&test_invocation(vec![array.clone(), DexValue::Int(0)])),
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&size, Some(&test_invocation(vec![array]))),
        Some(DexExecutionResult::Int(0))
    );
}

#[test]
fn builtin_autofill_value_for_text_stores_trimmed_text_payload() {
    let program = DexProgram::new(Vec::new());
    let for_text = DexMethodId {
        class_descriptor: "Landroid/view/autofill/AutofillValue;".to_owned(),
        name: "forText".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Landroid/view/autofill/AutofillValue;".to_owned(),
            parameters: vec!["Ljava/lang/CharSequence;".to_owned()],
        },
    };
    let type_field = DexFieldId {
        class_descriptor: "Landroid/view/autofill/AutofillValue;".to_owned(),
        type_descriptor: "I".to_owned(),
        name: "mType".to_owned(),
    };
    let value_field = DexFieldId {
        class_descriptor: "Landroid/view/autofill/AutofillValue;".to_owned(),
        type_descriptor: "Ljava/lang/Object;".to_owned(),
        name: "mValue".to_owned(),
    };

    let Some(DexExecutionResult::Value(text_value)) = program.invoke_builtin_method(
        &for_text,
        Some(&test_invocation(vec![DexValue::String(std::rc::Rc::from(
            " \tMeditation\n",
        ))])),
    ) else {
        panic!("AutofillValue.forText should return a value object");
    };
    assert_eq!(
        value_class_descriptor(&text_value),
        Some("Landroid/view/autofill/AutofillValue;")
    );
    assert_eq!(
        program.read_stored_instance_field(&text_value, &type_field),
        Some(DexValue::Int(1))
    );
    assert_eq!(
        program.read_stored_instance_field(&text_value, &value_field),
        Some(DexValue::String(std::rc::Rc::from("Meditation")))
    );

    let Some(DexExecutionResult::Value(null_value)) =
        program.invoke_builtin_method(&for_text, Some(&test_invocation(vec![DexValue::Null])))
    else {
        panic!("AutofillValue.forText(null) should return a value object");
    };
    assert_eq!(
        program.read_stored_instance_field(&null_value, &value_field),
        Some(DexValue::Null)
    );

    let char_sequence = DexValue::Object(DexObjectRef {
        id: 41,
        class_descriptor: std::rc::Rc::from("Landroid/text/SpannableString;"),
    });
    let Some(DexExecutionResult::Value(object_value)) = program.invoke_builtin_method(
        &for_text,
        Some(&test_invocation(vec![char_sequence.clone()])),
    ) else {
        panic!("AutofillValue.forText(object CharSequence) should return a value object");
    };
    assert_eq!(
        program.read_stored_instance_field(&object_value, &value_field),
        Some(char_sequence)
    );
}

#[test]
fn builtin_autofill_value_for_toggle_stores_boxed_boolean_payload() {
    let program = DexProgram::new(Vec::new());
    let for_toggle = DexMethodId {
        class_descriptor: "Landroid/view/autofill/AutofillValue;".to_owned(),
        name: "forToggle".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LZ".to_owned(),
            return_type: "Landroid/view/autofill/AutofillValue;".to_owned(),
            parameters: vec!["Z".to_owned()],
        },
    };
    let type_field = DexFieldId {
        class_descriptor: "Landroid/view/autofill/AutofillValue;".to_owned(),
        type_descriptor: "I".to_owned(),
        name: "mType".to_owned(),
    };
    let value_field = DexFieldId {
        class_descriptor: "Landroid/view/autofill/AutofillValue;".to_owned(),
        type_descriptor: "Ljava/lang/Object;".to_owned(),
        name: "mValue".to_owned(),
    };

    let Some(DexExecutionResult::Value(toggle_value)) =
        program.invoke_builtin_method(&for_toggle, Some(&test_invocation(vec![DexValue::Int(1)])))
    else {
        panic!("AutofillValue.forToggle should return a value object");
    };
    assert_eq!(
        program.read_stored_instance_field(&toggle_value, &type_field),
        Some(DexValue::Int(2))
    );
    assert_eq!(
        program.read_stored_instance_field(&toggle_value, &value_field),
        Some(DexValue::Reference(std::rc::Rc::from(
            "Ljava/lang/Boolean;:true"
        )))
    );
}

#[test]
fn builtin_autofill_manager_notify_view_visibility_changed_accepts_view_id_and_visibility() {
    let program = DexProgram::new(Vec::new());
    let notify = DexMethodId {
        class_descriptor: "Landroid/view/autofill/AutofillManager;".to_owned(),
        name: "notifyViewVisibilityChanged".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VLIZ".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "Landroid/view/View;".to_owned(),
                "I".to_owned(),
                "Z".to_owned(),
            ],
        },
    };
    let manager = DexValue::Reference(std::rc::Rc::from(
        "Landroid/view/autofill/AutofillManager;:autofill",
    ));
    let view = DexValue::Object(DexObjectRef {
        id: 51,
        class_descriptor: std::rc::Rc::from("Landroid/view/View;"),
    });

    assert_eq!(
        program.invoke_builtin_method(
            &notify,
            Some(&test_invocation(vec![
                manager.clone(),
                view,
                DexValue::Int(7),
                DexValue::Int(1),
            ])),
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &notify,
            Some(&test_invocation(vec![
                manager,
                DexValue::Null,
                DexValue::Int(7),
                DexValue::Int(1),
            ])),
        ),
        None
    );
}

#[test]
fn builtin_autofill_manager_notify_value_changed_accepts_virtual_value() {
    let program = DexProgram::new(Vec::new());
    let notify = DexMethodId {
        class_descriptor: "Landroid/view/autofill/AutofillManager;".to_owned(),
        name: "notifyValueChanged".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VLIL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "Landroid/view/View;".to_owned(),
                "I".to_owned(),
                "Landroid/view/autofill/AutofillValue;".to_owned(),
            ],
        },
    };
    let manager = DexValue::Reference(std::rc::Rc::from(
        "Landroid/view/autofill/AutofillManager;:autofill",
    ));
    let view = DexValue::Object(DexObjectRef {
        id: 52,
        class_descriptor: std::rc::Rc::from("Landroid/view/View;"),
    });
    let value = DexValue::Object(DexObjectRef {
        id: 53,
        class_descriptor: std::rc::Rc::from("Landroid/view/autofill/AutofillValue;"),
    });

    assert_eq!(
        program.invoke_builtin_method(
            &notify,
            Some(&test_invocation(vec![
                manager.clone(),
                view.clone(),
                DexValue::Int(9),
                value,
            ])),
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &notify,
            Some(&test_invocation(vec![
                manager,
                view,
                DexValue::Int(9),
                DexValue::Null,
            ])),
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &notify,
            Some(&test_invocation(vec![
                DexValue::Reference(std::rc::Rc::from(
                    "Landroid/view/autofill/AutofillManager;:autofill"
                )),
                DexValue::Null,
                DexValue::Int(9),
                DexValue::Null,
            ])),
        ),
        None
    );
}

#[test]
fn builtin_autofill_manager_notify_view_entered_accepts_virtual_bounds() {
    let program = DexProgram::new(Vec::new());
    let notify = DexMethodId {
        class_descriptor: "Landroid/view/autofill/AutofillManager;".to_owned(),
        name: "notifyViewEntered".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VLIL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "Landroid/view/View;".to_owned(),
                "I".to_owned(),
                "Landroid/graphics/Rect;".to_owned(),
            ],
        },
    };
    let manager = DexValue::Reference(std::rc::Rc::from(
        "Landroid/view/autofill/AutofillManager;:autofill",
    ));
    let view = DexValue::Object(DexObjectRef {
        id: 54,
        class_descriptor: std::rc::Rc::from("Landroid/view/View;"),
    });
    let bounds = DexValue::Object(DexObjectRef {
        id: 55,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Rect;"),
    });

    assert_eq!(
        program.invoke_builtin_method(
            &notify,
            Some(&test_invocation(vec![
                manager.clone(),
                view.clone(),
                DexValue::Int(12),
                bounds,
            ])),
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &notify,
            Some(&test_invocation(vec![
                manager,
                view,
                DexValue::Int(12),
                DexValue::Null,
            ])),
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &notify,
            Some(&test_invocation(vec![
                DexValue::Reference(std::rc::Rc::from(
                    "Landroid/view/autofill/AutofillManager;:autofill"
                )),
                DexValue::Null,
                DexValue::Int(12),
                DexValue::Null,
            ])),
        ),
        None
    );
}

#[test]
fn builtin_input_method_manager_restart_input_and_show_soft_input_accept_view() {
    let program = DexProgram::new(Vec::new());
    let restart_input = DexMethodId {
        class_descriptor: "Landroid/view/inputmethod/InputMethodManager;".to_owned(),
        name: "restartInput".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/view/View;".to_owned()],
        },
    };
    let show_soft_input = DexMethodId {
        class_descriptor: "Landroid/view/inputmethod/InputMethodManager;".to_owned(),
        name: "showSoftInput".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZLI".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Landroid/view/View;".to_owned(), "I".to_owned()],
        },
    };
    let manager = DexValue::Reference(std::rc::Rc::from(
        "Landroid/content/SystemService;:input_method",
    ));
    let view = DexValue::Object(DexObjectRef {
        id: 56,
        class_descriptor: std::rc::Rc::from("Landroid/view/View;"),
    });

    assert_eq!(
        program.invoke_builtin_method(
            &restart_input,
            Some(&test_invocation(vec![manager.clone(), view]))
        ),
        Some(DexExecutionResult::Void)
    );
    let view = DexValue::Object(DexObjectRef {
        id: 57,
        class_descriptor: std::rc::Rc::from("Landroid/view/View;"),
    });
    assert_eq!(
        program.invoke_builtin_method(
            &show_soft_input,
            Some(&test_invocation(vec![
                manager.clone(),
                view,
                DexValue::Int(0)
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &restart_input,
            Some(&test_invocation(vec![manager.clone(), DexValue::Null]))
        ),
        None
    );
    assert_eq!(
        program.invoke_builtin_method(
            &show_soft_input,
            Some(&test_invocation(vec![
                manager,
                DexValue::Null,
                DexValue::Int(0)
            ]))
        ),
        None
    );
}

#[test]
fn builtin_sparse_long_array_stores_long_values_by_int_key() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Landroid/util/SparseLongArray;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let put = DexMethodId {
        class_descriptor: "Landroid/util/SparseLongArray;".to_owned(),
        name: "put".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VIJ".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned(), "J".to_owned()],
        },
    };
    let get = DexMethodId {
        class_descriptor: "Landroid/util/SparseLongArray;".to_owned(),
        name: "get".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "JI".to_owned(),
            return_type: "J".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let get_with_default = DexMethodId {
        class_descriptor: "Landroid/util/SparseLongArray;".to_owned(),
        name: "get".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "JIJ".to_owned(),
            return_type: "J".to_owned(),
            parameters: vec!["I".to_owned(), "J".to_owned()],
        },
    };
    let value_at = DexMethodId {
        class_descriptor: "Landroid/util/SparseLongArray;".to_owned(),
        name: "valueAt".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "JI".to_owned(),
            return_type: "J".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let key_at = DexMethodId {
        class_descriptor: "Landroid/util/SparseLongArray;".to_owned(),
        name: "keyAt".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "II".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let index_of_key = DexMethodId {
        class_descriptor: "Landroid/util/SparseLongArray;".to_owned(),
        name: "indexOfKey".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "II".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let remove = DexMethodId {
        class_descriptor: "Landroid/util/SparseLongArray;".to_owned(),
        name: "delete".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let size = DexMethodId {
        class_descriptor: "Landroid/util/SparseLongArray;".to_owned(),
        name: "size".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let array = DexValue::Reference(std::rc::Rc::from("sparse-long-array"));

    assert_eq!(
        program.invoke_builtin_method(&constructor, Some(&test_invocation(vec![array.clone()]))),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_with_default,
            Some(&test_invocation(vec![
                array.clone(),
                DexValue::Int(7),
                DexValue::Long(99),
                DexValue::Long(99),
            ])),
        ),
        Some(DexExecutionResult::Value(DexValue::Long(99)))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &put,
            Some(&test_invocation(vec![
                array.clone(),
                DexValue::Int(42),
                DexValue::Long(123),
                DexValue::Long(123),
            ])),
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&size, Some(&test_invocation(vec![array.clone()]))),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get,
            Some(&test_invocation(vec![array.clone(), DexValue::Int(42)])),
        ),
        Some(DexExecutionResult::Value(DexValue::Long(123)))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &key_at,
            Some(&test_invocation(vec![array.clone(), DexValue::Int(0)])),
        ),
        Some(DexExecutionResult::Int(42))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &index_of_key,
            Some(&test_invocation(vec![array.clone(), DexValue::Int(42)])),
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &index_of_key,
            Some(&test_invocation(vec![array.clone(), DexValue::Int(7)])),
        ),
        Some(DexExecutionResult::Int(-1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &value_at,
            Some(&test_invocation(vec![array.clone(), DexValue::Int(0)])),
        ),
        Some(DexExecutionResult::Value(DexValue::Long(123)))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &remove,
            Some(&test_invocation(vec![array.clone(), DexValue::Int(42)])),
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&size, Some(&test_invocation(vec![array]))),
        Some(DexExecutionResult::Int(0))
    );
}

#[test]
fn builtin_sparse_boolean_array_stores_boolean_values_by_int_key() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Landroid/util/SparseBooleanArray;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let put = DexMethodId {
        class_descriptor: "Landroid/util/SparseBooleanArray;".to_owned(),
        name: "put".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VIZ".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned(), "Z".to_owned()],
        },
    };
    let get = DexMethodId {
        class_descriptor: "Landroid/util/SparseBooleanArray;".to_owned(),
        name: "get".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZI".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let get_with_default = DexMethodId {
        class_descriptor: "Landroid/util/SparseBooleanArray;".to_owned(),
        name: "get".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZIZ".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["I".to_owned(), "Z".to_owned()],
        },
    };
    let value_at = DexMethodId {
        class_descriptor: "Landroid/util/SparseBooleanArray;".to_owned(),
        name: "valueAt".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZI".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let key_at = DexMethodId {
        class_descriptor: "Landroid/util/SparseBooleanArray;".to_owned(),
        name: "keyAt".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "II".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let remove = DexMethodId {
        class_descriptor: "Landroid/util/SparseBooleanArray;".to_owned(),
        name: "delete".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let size = DexMethodId {
        class_descriptor: "Landroid/util/SparseBooleanArray;".to_owned(),
        name: "size".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let array = DexValue::Reference(std::rc::Rc::from("sparse-boolean-array"));

    assert_eq!(
        program.invoke_builtin_method(&constructor, Some(&test_invocation(vec![array.clone()]))),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_with_default,
            Some(&test_invocation(vec![
                array.clone(),
                DexValue::Int(7),
                DexValue::Int(1),
            ])),
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &put,
            Some(&test_invocation(vec![
                array.clone(),
                DexValue::Int(42),
                DexValue::Int(1),
            ])),
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&size, Some(&test_invocation(vec![array.clone()]))),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get,
            Some(&test_invocation(vec![array.clone(), DexValue::Int(42)])),
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &key_at,
            Some(&test_invocation(vec![array.clone(), DexValue::Int(0)])),
        ),
        Some(DexExecutionResult::Int(42))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &value_at,
            Some(&test_invocation(vec![array.clone(), DexValue::Int(0)])),
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &remove,
            Some(&test_invocation(vec![array.clone(), DexValue::Int(42)])),
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&size, Some(&test_invocation(vec![array]))),
        Some(DexExecutionResult::Int(0))
    );
}

// cite: frameworks-base Vibrator.java#areAllPrimitivesSupported, getInfo
// cite: frameworks-base VibratorInfo.java#EMPTY_VIBRATOR_INFO, isPrimitiveSupported
#[test]
fn vibrator_are_all_primitives_supported_follows_empty_vibrator_info() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/os/Vibrator;".to_owned(),
        name: "areAllPrimitivesSupported".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["[I".to_owned()],
        },
    };
    let vibrator = DexValue::Reference(std::rc::Rc::from("Landroid/os/Vibrator;:vibrator"));
    let empty = program.allocate_filled_array("[I".to_owned(), Vec::new());
    let single = program.allocate_filled_array("[I".to_owned(), vec![DexValue::Int(1)]);

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![vibrator.clone(), empty]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![vibrator, single]))),
        Some(DexExecutionResult::Int(0))
    );
}

// cite: frameworks-base Bundle.java#putCharSequence, getCharSequence
#[test]
fn bundle_char_sequence_round_trips() {
    let program = DexProgram::new(Vec::new());
    let bundle = program.allocate_object("Landroid/os/Bundle;".to_owned());
    let put = DexMethodId {
        class_descriptor: "Landroid/os/Bundle;".to_owned(),
        name: "putCharSequence".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VLL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "Ljava/lang/String;".to_owned(),
                "Ljava/lang/CharSequence;".to_owned(),
            ],
        },
    };
    let get = DexMethodId {
        class_descriptor: "Landroid/os/Bundle;".to_owned(),
        name: "getCharSequence".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/lang/CharSequence;".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned()],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(
            &get,
            Some(&test_invocation(vec![
                bundle.clone(),
                DexValue::String(std::rc::Rc::from("title")),
            ])),
        ),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &put,
            Some(&test_invocation(vec![
                bundle.clone(),
                DexValue::String(std::rc::Rc::from("title")),
                DexValue::String(std::rc::Rc::from("Candle")),
            ])),
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get,
            Some(&test_invocation(vec![
                bundle,
                DexValue::String(std::rc::Rc::from("title")),
            ])),
        ),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("Candle")
        )))
    );
}

// cite: frameworks-base AccessibilityManager.java#getEnabledAccessibilityServiceList
#[test]
fn accessibility_manager_enabled_service_list_is_empty_without_service() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/view/accessibility/AccessibilityManager;".to_owned(),
        name: "getEnabledAccessibilityServiceList".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LI".to_owned(),
            return_type: "Ljava/util/List;".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let manager = DexValue::Reference(std::rc::Rc::from(
        "Landroid/view/accessibility/AccessibilityManager;:accessibility",
    ));

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![manager, DexValue::Int(-1)]))
        ),
        Some(DexExecutionResult::Value(program.collections_empty_list()))
    );
}

// cite: frameworks-base MimeTypeMap.java#getSingleton, getMimeTypeFromExtension
#[test]
fn mime_type_map_singleton_resolves_known_and_unknown_extensions() {
    let program = DexProgram::new(Vec::new());
    let get_singleton = DexMethodId {
        class_descriptor: "Landroid/webkit/MimeTypeMap;".to_owned(),
        name: "getSingleton".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/webkit/MimeTypeMap;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get_mime_type = DexMethodId {
        class_descriptor: "Landroid/webkit/MimeTypeMap;".to_owned(),
        name: "getMimeTypeFromExtension".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned()],
        },
    };
    let Some(DexExecutionResult::Value(singleton)) =
        program.invoke_builtin_method(&get_singleton, Some(&test_invocation(Vec::new())))
    else {
        panic!("MimeTypeMap.getSingleton should return a value");
    };

    assert_eq!(
        program.invoke_builtin_method(
            &get_mime_type,
            Some(&test_invocation(vec![
                singleton.clone(),
                DexValue::String(std::rc::Rc::from("TXT")),
            ]))
        ),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("text/plain")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_mime_type,
            Some(&test_invocation(vec![
                singleton,
                DexValue::String(std::rc::Rc::from("not-a-real-extension")),
            ]))
        ),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
}

// cite: frameworks-base MimeTypeMap.java#hasMimeType, hasExtension, getExtensionFromMimeType
#[test]
fn mime_type_map_has_mime_type_extension_and_reverse_lookup_round_trip() {
    let program = DexProgram::new(Vec::new());
    let has_mime_type = DexMethodId {
        class_descriptor: "Landroid/webkit/MimeTypeMap;".to_owned(),
        name: "hasMimeType".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned()],
        },
    };
    let has_extension = DexMethodId {
        class_descriptor: "Landroid/webkit/MimeTypeMap;".to_owned(),
        name: "hasExtension".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned()],
        },
    };
    let get_extension = DexMethodId {
        class_descriptor: "Landroid/webkit/MimeTypeMap;".to_owned(),
        name: "getExtensionFromMimeType".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned()],
        },
    };

    let singleton =
        DexValue::Reference(std::rc::Rc::from("Landroid/webkit/MimeTypeMap;.singleton"));

    assert_eq!(
        program.invoke_builtin_method(
            &has_mime_type,
            Some(&test_invocation(vec![
                singleton.clone(),
                DexValue::String(std::rc::Rc::from("image/png")),
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &has_mime_type,
            Some(&test_invocation(vec![
                singleton.clone(),
                DexValue::String(std::rc::Rc::from("application/not-a-real-type")),
            ]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &has_extension,
            Some(&test_invocation(vec![
                singleton.clone(),
                DexValue::String(std::rc::Rc::from("png")),
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_extension,
            Some(&test_invocation(vec![
                singleton,
                DexValue::String(std::rc::Rc::from("image/png")),
            ]))
        ),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("png")
        )))
    );
}

fn log_get_stack_trace_string_method() -> DexMethodId {
    DexMethodId {
        class_descriptor: "Landroid/util/Log;".to_owned(),
        name: "getStackTraceString".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: vec!["Ljava/lang/Throwable;".to_owned()],
        },
    }
}

fn throwable_field(name: &str, type_descriptor: &str) -> DexFieldId {
    DexFieldId {
        class_descriptor: "Ljava/lang/Throwable;".to_owned(),
        type_descriptor: type_descriptor.to_owned(),
        name: name.to_owned(),
    }
}

fn throwable_with(
    program: &DexProgram,
    class_descriptor: &str,
    message: &str,
    cause: Option<DexValue>,
) -> DexValue {
    let throwable = program.allocate_object(class_descriptor.to_owned());
    program.write_stored_instance_field(
        &throwable,
        &throwable_field("detailMessage", "Ljava/lang/String;"),
        DexValue::String(std::rc::Rc::from(message)),
    );
    if let Some(cause) = cause {
        program.write_stored_instance_field(
            &throwable,
            &throwable_field("cause", "Ljava/lang/Throwable;"),
            cause,
        );
    }

    throwable
}

fn stack_trace_string(program: &DexProgram, throwable: DexValue) -> Option<DexExecutionResult> {
    program.invoke_builtin_method(
        &log_get_stack_trace_string_method(),
        Some(&test_invocation(vec![throwable])),
    )
}

fn string_result(text: &str) -> Option<DexExecutionResult> {
    Some(DexExecutionResult::Value(DexValue::String(
        std::rc::Rc::from(text),
    )))
}

#[test]
fn frameworks_base_log_get_stack_trace_string_returns_empty_for_null() {
    // cite: frameworks-base Log.java#getStackTraceString
    let program = DexProgram::new(Vec::new());

    assert_eq!(stack_trace_string(&program, DexValue::Null), string_result(""));
}

#[test]
fn frameworks_base_log_get_stack_trace_string_prints_header_and_cause() {
    // cite: frameworks-base Log.java#getStackTraceString
    // cite: libcore Throwable.java#printStackTrace, printEnclosedStackTrace
    let program = DexProgram::new(Vec::new());
    let cause = throwable_with(&program, "Ljava/lang/IllegalStateException;", "b", None);
    let outer = throwable_with(&program, "Ljava/lang/RuntimeException;", "a", Some(cause));

    assert_eq!(
        stack_trace_string(&program, outer),
        string_result(
            "java.lang.RuntimeException: a\nCaused by: java.lang.IllegalStateException: b\n"
        )
    );
}

#[test]
fn frameworks_base_log_get_stack_trace_string_returns_empty_for_unknown_host_in_cause_chain() {
    // cite: frameworks-base Log.java#getStackTraceString
    let program = DexProgram::new(Vec::new());
    let host = throwable_with(&program, "Ljava/net/UnknownHostException;", "h", None);
    let outer = throwable_with(&program, "Ljava/lang/RuntimeException;", "a", Some(host));

    assert_eq!(stack_trace_string(&program, outer), string_result(""));
}

#[test]
fn libcore_throwable_print_stack_trace_marks_a_cause_cycle_as_circular() {
    // cite: libcore Throwable.java#printEnclosedStackTrace
    let program = DexProgram::new(Vec::new());
    let first = throwable_with(&program, "Ljava/lang/RuntimeException;", "a", None);
    let second = throwable_with(
        &program,
        "Ljava/lang/IllegalStateException;",
        "b",
        Some(first.clone()),
    );
    program.write_stored_instance_field(
        &first,
        &throwable_field("cause", "Ljava/lang/Throwable;"),
        second,
    );

    assert_eq!(
        stack_trace_string(&program, first),
        string_result(
            "java.lang.RuntimeException: a\nCaused by: java.lang.IllegalStateException: b\nCaused by: [CIRCULAR REFERENCE: java.lang.RuntimeException: a]\n"
        )
    );
}

#[test]
fn frameworks_base_log_get_stack_trace_string_rejects_a_non_throwable() {
    // cite: frameworks-base Log.java#getStackTraceString
    let program = DexProgram::new(Vec::new());
    let object = program.allocate_object("Ljava/util/ArrayList;".to_owned());

    assert_eq!(stack_trace_string(&program, object), None);
    assert_eq!(stack_trace_string(&program, DexValue::Int(7)), None);
}

#[test]
fn frameworks_base_environment_get_data_directory_returns_the_default_data_root() {
    // cite: frameworks-base Environment.java#getDataDirectory, DIR_ANDROID_DATA_PATH
    let program = DexProgram::new(Vec::new());
    let method = builtin_method("Landroid/os/Environment;", "getDataDirectory", "Ljava/io/File;", &[]);

    assert_eq!(
        program.invoke_builtin_method(&method, None),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Ljava/io/File;:/data")
        )))
    );
}
