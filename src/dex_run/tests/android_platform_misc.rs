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
fn builtin_animation_utils_current_time_follows_the_main_thread_clock() {
    // cite: frameworks-base AnimationUtils.java#currentAnimationTimeMillis
    let program = DexProgram::new(Vec::new());
    let method = builtin_method(
        "Landroid/view/animation/AnimationUtils;",
        "currentAnimationTimeMillis",
        "J",
        &[],
    );
    program.runtime.borrow_mut().main_thread_uptime_millis = 1234;

    assert_eq!(
        program.invoke_builtin_method(&method, None),
        Some(DexExecutionResult::Value(DexValue::Long(1234)))
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

    assert_eq!(
        stack_trace_string(&program, DexValue::Null),
        string_result("")
    );
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
    let method = builtin_method(
        "Landroid/os/Environment;",
        "getDataDirectory",
        "Ljava/io/File;",
        &[],
    );

    assert_eq!(
        program.invoke_builtin_method(&method, None),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Ljava/io/File;:/data")
        )))
    );
}

fn web_view_set_data_directory_suffix_method() -> DexMethodId {
    DexMethodId {
        class_descriptor: "Landroid/webkit/WebView;".to_owned(),
        name: "setDataDirectorySuffix".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned()],
        },
    }
}

// cite: frameworks-base WebView.java#setDataDirectorySuffix
// cite: frameworks-base WebViewFactory.java#setDataDirectorySuffix, sDataDirectorySuffix
#[test]
fn web_view_set_data_directory_suffix_stores_suffix_in_web_view_factory() {
    let program = DexProgram::new(Vec::new());
    let method = web_view_set_data_directory_suffix_method();
    let suffix = DexValue::String(std::rc::Rc::from("push"));

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![suffix.clone()]))),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_static_field(&DexFieldId {
            class_descriptor: "Landroid/webkit/WebViewFactory;".to_owned(),
            type_descriptor: "Ljava/lang/String;".to_owned(),
            name: "sDataDirectorySuffix".to_owned(),
        }),
        Some(suffix)
    );
}

// cite: frameworks-base WebViewFactory.java#setDataDirectorySuffix
#[test]
fn web_view_set_data_directory_suffix_refuses_path_separator_and_null() {
    let program = DexProgram::new(Vec::new());
    let method = web_view_set_data_directory_suffix_method();

    let separator = program.invoke_builtin_method(
        &method,
        Some(&test_invocation(vec![DexValue::String(std::rc::Rc::from(
            "a/b",
        ))])),
    );
    let Some(DexExecutionResult::Throw(exception)) = separator else {
        panic!("a suffix with a path separator should throw");
    };
    assert_eq!(
        thrown_detail_message(&program, &exception).as_deref(),
        Some("Suffix a/b contains a path separator")
    );

    for null in [DexValue::Null, DexValue::Int(0)] {
        assert!(matches!(
            program.invoke_builtin_method(&method, Some(&test_invocation(vec![null]))),
            Some(DexExecutionResult::Throw(_))
        ));
    }
}

#[test]
fn frameworks_binder_get_calling_uid_outside_a_transaction_is_the_process_uid() {
    // cite: frameworks-base Binder.java#getCallingUid
    let program = DexProgram::new(Vec::new());
    let calling_uid = builtin_method("Landroid/os/Binder;", "getCallingUid", "I", &[]);
    let my_uid = builtin_method("Landroid/os/Process;", "myUid", "I", &[]);

    assert_eq!(
        program.invoke_builtin_method(&calling_uid, Some(&test_invocation(Vec::new()))),
        program.invoke_builtin_method(&my_uid, Some(&test_invocation(Vec::new())))
    );
}

#[test]
// cite: frameworks-base ActivityManager.java#getHistoricalProcessStartReasons
// cite: frameworks-base ActivityManagerService.java#getHistoricalProcessStartReasons
fn frameworks_activity_manager_reports_no_historical_process_start_reasons() {
    let program = DexProgram::new(Vec::new());
    let reasons = builtin_method(
        "Landroid/app/ActivityManager;",
        "getHistoricalProcessStartReasons",
        "Ljava/util/List;",
        &["I"],
    );
    let size = builtin_method("Ljava/util/List;", "size", "I", &[]);
    let activity_manager = DexValue::Reference(std::rc::Rc::from("Landroid/app/ActivityManager;"));

    for max_num in [0, 1] {
        let Some(DexExecutionResult::Value(list)) = program.invoke_builtin_method(
            &reasons,
            Some(&test_invocation(vec![
                activity_manager.clone(),
                DexValue::Int(max_num),
            ])),
        ) else {
            panic!("start reasons should be a list");
        };
        assert_eq!(
            program.invoke_builtin_method(&size, Some(&test_invocation(vec![list]))),
            Some(DexExecutionResult::Int(0))
        );
    }
}

#[test]
fn frameworks_web_view_client_constructor_returns_void() {
    // cite: frameworks-base WebViewClient.java#WebViewClient
    let program = DexProgram::new(Vec::new());
    let constructor = builtin_method("Landroid/webkit/WebViewClient;", "<init>", "V", &[]);
    let client = program.allocate_object("Landroid/webkit/WebViewClient;".to_owned());

    assert_eq!(
        program.invoke_builtin_method(&constructor, Some(&test_invocation(vec![client.clone()]))),
        Some(DexExecutionResult::Void)
    );
    assert!(program.value_matches_type(&client, "Landroid/webkit/WebViewClient;"));
}

#[test]
fn frameworks_display_metrics_constructor_returns_void() {
    // cite: frameworks-base DisplayMetrics.java#DisplayMetrics
    let program = DexProgram::new(Vec::new());
    let constructor = builtin_method("Landroid/util/DisplayMetrics;", "<init>", "V", &[]);
    let metrics = program.allocate_object("Landroid/util/DisplayMetrics;".to_owned());

    assert_eq!(
        program.invoke_builtin_method(&constructor, Some(&test_invocation(vec![metrics.clone()]))),
        Some(DexExecutionResult::Void)
    );
    assert!(program.value_matches_type(&metrics, "Landroid/util/DisplayMetrics;"));
}

// cite: frameworks-base Pair.java#Pair
#[test]
fn builtin_pair_constructor_stores_first_and_second_fields() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Landroid/util/Pair;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VLL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "Ljava/lang/Object;".to_owned(),
                "Ljava/lang/Object;".to_owned(),
            ],
        },
    };
    let pair = program.allocate_object("Landroid/util/Pair;".to_owned());
    let first = DexValue::String(std::rc::Rc::from("key"));
    let second = DexValue::String(std::rc::Rc::from("value"));

    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![
                pair.clone(),
                first.clone(),
                second.clone()
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    for (name, expected) in [("first", first), ("second", second)] {
        assert_eq!(
            program.read_stored_instance_field(
                &pair,
                &DexFieldId {
                    class_descriptor: "Landroid/util/Pair;".to_owned(),
                    type_descriptor: "Ljava/lang/Object;".to_owned(),
                    name: name.to_owned(),
                }
            ),
            Some(expected)
        );
    }
}

// cite: frameworks-base Settings.java#getString
#[test]
fn builtin_settings_secure_get_string_returns_android_id() {
    let program = DexProgram::with_package_name(Vec::new(), Some("com.example".to_owned()));
    let method = DexMethodId {
        class_descriptor: "Landroid/provider/Settings$Secure;".to_owned(),
        name: "getString".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LLL".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: vec![
                "Landroid/content/ContentResolver;".to_owned(),
                "Ljava/lang/String;".to_owned(),
            ],
        },
    };
    let resolver = DexValue::Reference(std::rc::Rc::from("resolver"));
    let android_id = program.invoke_builtin_method(
        &method,
        Some(&test_invocation(vec![
            resolver.clone(),
            DexValue::String(std::rc::Rc::from("android_id")),
        ])),
    );

    assert!(matches!(
        android_id,
        Some(DexExecutionResult::Value(DexValue::String(value))) if value.len() == 16
    ));
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                resolver,
                DexValue::String(std::rc::Rc::from("unknown"))
            ]))
        ),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
}

// cite: frameworks-base BaseBundle.java#getString, putString, putBoolean
#[test]
fn bundle_get_string_returns_the_default_for_missing_or_non_string_values() {
    let program = DexProgram::new(Vec::new());
    let class = "Landroid/os/BaseBundle;";
    let put_string = builtin_method(
        class,
        "putString",
        "V",
        &["Ljava/lang/String;", "Ljava/lang/String;"],
    );
    let put_boolean = builtin_method(class, "putBoolean", "V", &["Ljava/lang/String;", "Z"]);
    let get_string = builtin_method(
        class,
        "getString",
        "Ljava/lang/String;",
        &["Ljava/lang/String;", "Ljava/lang/String;"],
    );
    let bundle = program.allocate_object("Landroid/os/Bundle;".to_owned());
    let string = |value: &str| DexValue::String(std::rc::Rc::from(value));
    let call = |method: &DexMethodId, mut arguments: Vec<DexValue>| {
        arguments.insert(0, bundle.clone());
        program.invoke_builtin_method(method, Some(&test_invocation(arguments)))
    };
    call(&put_string, vec![string("tab"), string("home")]);
    call(&put_boolean, vec![string("flag"), DexValue::Int(1)]);

    assert_eq!(
        call(&get_string, vec![string("tab"), DexValue::Int(0)]),
        Some(DexExecutionResult::Value(string("home")))
    );
    assert_eq!(
        call(&get_string, vec![string("missing"), DexValue::Int(0)]),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    assert_eq!(
        call(&get_string, vec![string("flag"), string("fallback")]),
        Some(DexExecutionResult::Value(string("fallback")))
    );
}

// cite: frameworks-base Color.java#alpha, red, green, blue, rgb
#[test]
fn builtin_color_channels_follow_aosp_bit_layout() {
    let program = DexProgram::new(Vec::new());
    let channel = |name: &str, color: i32| {
        program.invoke_builtin_method(
            &builtin_method("Landroid/graphics/Color;", name, "I", &["I"]),
            Some(&test_invocation(vec![DexValue::Int(color)])),
        )
    };
    let color = 0x80_12_34_56u32 as i32;

    assert_eq!(channel("alpha", color), Some(DexExecutionResult::Int(0x80)));
    assert_eq!(channel("red", color), Some(DexExecutionResult::Int(0x12)));
    assert_eq!(channel("green", color), Some(DexExecutionResult::Int(0x34)));
    assert_eq!(channel("blue", color), Some(DexExecutionResult::Int(0x56)));
    assert_eq!(
        program.invoke_builtin_method(
            &builtin_method("Landroid/graphics/Color;", "rgb", "I", &["I", "I", "I"]),
            Some(&test_invocation(vec![
                DexValue::Int(0x12),
                DexValue::Int(0x34),
                DexValue::Int(0x56)
            ]))
        ),
        Some(DexExecutionResult::Int(0xff12_3456u32 as i32))
    );
}

// cite: frameworks-base TelephonyManager.java#getSimCountryIso, getTelephonyProperty
#[test]
fn builtin_telephony_sim_country_iso_reads_the_sim_property_or_empty() {
    let program = DexProgram::new(Vec::new());
    let value = program.invoke_builtin_method(
        &builtin_method(
            "Landroid/telephony/TelephonyManager;",
            "getSimCountryIso",
            "Ljava/lang/String;",
            &[],
        ),
        Some(&test_invocation(vec![DexValue::Reference(
            std::rc::Rc::from("Landroid/telephony/TelephonyManager;:phone"),
        )])),
    );
    let expected = crate::native::bionic::props::get("gsm.sim.operator.iso-country")
        .and_then(|list| list.split(',').next().map(str::to_owned))
        .unwrap_or_default();

    assert_eq!(
        value,
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from(expected)
        )))
    );
}

// cite: frameworks-base AudioService.java#MAX_STREAM_VOLUME, ensureValidStreamType
#[test]
fn builtin_audio_manager_stream_max_volume_follows_audio_service_defaults() {
    let program = DexProgram::new(Vec::new());
    let max = |stream: i32| {
        program.invoke_builtin_method(
            &builtin_method(
                "Landroid/media/AudioManager;",
                "getStreamMaxVolume",
                "I",
                &["I"],
            ),
            Some(&test_invocation(vec![
                DexValue::Reference(std::rc::Rc::from("Landroid/media/AudioManager;:audio")),
                DexValue::Int(stream),
            ])),
        )
    };

    if crate::native::bionic::props::get("ro.config.media_vol_steps").is_none() {
        assert_eq!(max(3), Some(DexExecutionResult::Int(15)));
    }
    assert_eq!(max(1), Some(DexExecutionResult::Int(7)));
    assert!(matches!(max(99), Some(DexExecutionResult::Throw(_))));
}

// cite: frameworks-base AudioService.java#MAX_STREAM_VOLUME
// cite: frameworks-base AudioSystem.java#DEFAULT_STREAM_VOLUME
#[test]
fn builtin_audio_manager_stream_volume_uses_audio_service_defaults() {
    let program = DexProgram::new(Vec::new());
    let volume = |stream: i32| {
        program.invoke_builtin_method(
            &builtin_method(
                "Landroid/media/AudioManager;",
                "getStreamVolume",
                "I",
                &["I"],
            ),
            Some(&test_invocation(vec![
                DexValue::Reference(std::rc::Rc::from("Landroid/media/AudioManager;:audio")),
                DexValue::Int(stream),
            ])),
        )
    };
    let no_override = |name: &str| crate::native::bionic::props::get(name).is_none();

    if no_override("ro.config.media_vol_steps") && no_override("ro.config.media_vol_default") {
        assert_eq!(volume(3), Some(DexExecutionResult::Int(5)));
    }
    if no_override("ro.config.alarm_vol_steps") && no_override("ro.config.alarm_vol_default") {
        assert_eq!(volume(4), Some(DexExecutionResult::Int(6)));
    }
    assert_eq!(volume(2), Some(DexExecutionResult::Int(5)));
}
