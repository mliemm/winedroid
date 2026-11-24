use super::super::*;
use super::helpers::*;

#[test]
fn builtin_object_constructor_returns_void() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Ljava/lang/Object;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let equals = DexMethodId {
        class_descriptor: "Ljava/lang/Object;".to_owned(),
        name: "equals".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let to_string = DexMethodId {
        class_descriptor: "Ljava/lang/Object;".to_owned(),
        name: "toString".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let notify_all = DexMethodId {
        class_descriptor: "Ljava/lang/Object;".to_owned(),
        name: "notifyAll".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let same_invocation = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from("same")),
        DexValue::Reference(std::rc::Rc::from("same")),
    ]);
    let different_invocation = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from("left")),
        DexValue::Reference(std::rc::Rc::from("right")),
    ]);
    let to_string_invocation =
        test_invocation(vec![DexValue::String(std::rc::Rc::from("TraceTag"))]);
    let object = DexValue::Object(DexObjectRef {
        id: 7,
        class_descriptor: std::rc::Rc::from("Lcom/example/Thing;"),
    });
    let object_string = format!(
        "com.example.Thing@{:x}",
        stable_hash_i32(&map_identity_key(&object)) as u32
    );

    assert_eq!(
        program.invoke_builtin_method(&constructor, None),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&equals, Some(&same_invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&equals, Some(&different_invocation)),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&to_string, Some(&to_string_invocation)),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("TraceTag")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&to_string, Some(&test_invocation(vec![object]))),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from(object_string)
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &notify_all,
            Some(&test_invocation(vec![DexValue::Object(DexObjectRef {
                id: 8,
                class_descriptor: std::rc::Rc::from("Ljava/lang/Object;"),
            })]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&notify_all, Some(&test_invocation(vec![DexValue::Null]))),
        None
    );
}

#[test]
fn builtin_objects_to_string_returns_null_literal_or_object_string() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/util/Objects;".to_owned(),
        name: "toString".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let object = DexValue::Object(DexObjectRef {
        id: 8,
        class_descriptor: std::rc::Rc::from("Lcom/example/Thing;"),
    });
    let object_string = format!(
        "com.example.Thing@{:x}",
        stable_hash_i32(&map_identity_key(&object)) as u32
    );

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![DexValue::Null]))),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("null")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![object]))),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from(object_string)
        )))
    );
}

#[test]
fn builtin_comparable_compare_to_matches_modeled_platform_values() {
    let program = DexProgram::new(Vec::new());
    let compare_to = DexMethodId {
        class_descriptor: "Ljava/lang/Comparable;".to_owned(),
        name: "compareTo".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "IL".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let integer_constructor = DexMethodId {
        class_descriptor: "Ljava/lang/Integer;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let integer_object = DexValue::Object(DexObjectRef {
        id: 21,
        class_descriptor: std::rc::Rc::from("Ljava/lang/Integer;"),
    });

    assert_eq!(
        program.invoke_builtin_method(
            &compare_to,
            Some(&test_invocation(vec![
                DexValue::String(std::rc::Rc::from("alpha")),
                DexValue::String(std::rc::Rc::from("beta")),
            ]))
        ),
        Some(DexExecutionResult::Int(-1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &compare_to,
            Some(&test_invocation(vec![
                DexValue::Reference(std::rc::Rc::from("Ljava/lang/Integer;:12")),
                DexValue::Reference(std::rc::Rc::from("Ljava/lang/Integer;:3")),
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &compare_to,
            Some(&test_invocation(vec![
                DexValue::Reference(std::rc::Rc::from("Ljava/lang/Float;:80000000")),
                DexValue::Reference(std::rc::Rc::from("Ljava/lang/Float;:00000000")),
            ]))
        ),
        Some(DexExecutionResult::Int(-1))
    );

    assert_eq!(
        program.invoke_builtin_method(
            &integer_constructor,
            Some(&test_invocation(vec![
                integer_object.clone(),
                DexValue::Int(7),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &compare_to,
            Some(&test_invocation(vec![
                integer_object,
                DexValue::Reference(std::rc::Rc::from("Ljava/lang/Integer;:7")),
            ]))
        ),
        Some(DexExecutionResult::Int(0))
    );

    assert_eq!(
        program.invoke_builtin_method(
            &compare_to,
            Some(&test_invocation(vec![
                DexValue::Reference(std::rc::Rc::from("Ljava/lang/Integer;:1")),
                DexValue::Reference(std::rc::Rc::from("Ljava/lang/Long;:1")),
            ]))
        ),
        None
    );
}

#[test]
fn builtin_throwable_string_constructors_store_message_when_invoked() {
    let program = DexProgram::new(Vec::new());
    let illegal_state = DexMethodId {
        class_descriptor: "Ljava/lang/IllegalStateException;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned()],
        },
    };
    let runtime_exception = DexMethodId {
        class_descriptor: "Ljava/lang/RuntimeException;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let illegal_argument = DexMethodId {
        class_descriptor: "Ljava/lang/IllegalArgumentException;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned()],
        },
    };
    let cancellation = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/CancellationException;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned()],
        },
    };
    let no_such_element = DexMethodId {
        class_descriptor: "Ljava/util/NoSuchElementException;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned()],
        },
    };
    let error_with_cause = DexMethodId {
        class_descriptor: "Ljava/lang/Error;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VLL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "Ljava/lang/String;".to_owned(),
                "Ljava/lang/Throwable;".to_owned(),
            ],
        },
    };
    let init_cause = DexMethodId {
        class_descriptor: "Ljava/lang/Throwable;".to_owned(),
        name: "initCause".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/lang/Throwable;".to_owned(),
            parameters: vec!["Ljava/lang/Throwable;".to_owned()],
        },
    };
    let add_suppressed = DexMethodId {
        class_descriptor: "Ljava/lang/Throwable;".to_owned(),
        name: "addSuppressed".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/lang/Throwable;".to_owned()],
        },
    };
    let get_suppressed = DexMethodId {
        class_descriptor: "Ljava/lang/Throwable;".to_owned(),
        name: "getSuppressed".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "[Ljava/lang/Throwable;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let exception = DexValue::Object(DexObjectRef {
        id: 1,
        class_descriptor: std::rc::Rc::from("Ljava/util/concurrent/CancellationException;"),
    });
    let argument_exception = DexValue::Object(DexObjectRef {
        id: 2,
        class_descriptor: std::rc::Rc::from("Ljava/lang/IllegalArgumentException;"),
    });
    let fatal_error = DexValue::Object(DexObjectRef {
        id: 3,
        class_descriptor: std::rc::Rc::from("Ljava/lang/Error;"),
    });
    let missing_element = DexValue::Object(DexObjectRef {
        id: 4,
        class_descriptor: std::rc::Rc::from("Ljava/util/NoSuchElementException;"),
    });
    let message_field = DexFieldId {
        class_descriptor: "Ljava/lang/Throwable;".to_owned(),
        type_descriptor: "Ljava/lang/String;".to_owned(),
        name: "detailMessage".to_owned(),
    };
    let cause_field = DexFieldId {
        class_descriptor: "Ljava/lang/Throwable;".to_owned(),
        type_descriptor: "Ljava/lang/Throwable;".to_owned(),
        name: "cause".to_owned(),
    };
    let suppressed_field = DexFieldId {
        class_descriptor: "Ljava/lang/Throwable;".to_owned(),
        type_descriptor: "[Ljava/lang/Throwable;".to_owned(),
        name: "suppressedExceptions".to_owned(),
    };

    assert_eq!(
        program.invoke_builtin_method(&illegal_state, None),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&runtime_exception, None),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &illegal_argument,
            Some(&test_invocation(vec![
                argument_exception.clone(),
                DexValue::String(std::rc::Rc::from("bad argument")),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &cancellation,
            Some(&test_invocation(vec![
                exception.clone(),
                DexValue::String(std::rc::Rc::from("cancelled")),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &no_such_element,
            Some(&test_invocation(vec![
                missing_element.clone(),
                DexValue::String(std::rc::Rc::from("missing")),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&exception, &message_field),
        Some(DexValue::String(std::rc::Rc::from("cancelled")))
    );
    let subclass_cancellation = DexValue::Object(DexObjectRef {
        id: 5,
        class_descriptor: std::rc::Rc::from("Lrv1;"),
    });
    assert_eq!(
        program.invoke_builtin_method(
            &cancellation,
            Some(&test_invocation(vec![
                subclass_cancellation.clone(),
                DexValue::String(std::rc::Rc::from("subclass cancelled")),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&subclass_cancellation, &message_field),
        Some(DexValue::String(std::rc::Rc::from("subclass cancelled")))
    );
    let object_backed_message =
        DexValue::Reference(std::rc::Rc::from("Ljava/lang/String;:message"));
    assert_eq!(
        program.invoke_builtin_method(
            &cancellation,
            Some(&test_invocation(vec![
                exception.clone(),
                object_backed_message.clone(),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&exception, &message_field),
        Some(object_backed_message)
    );
    assert_eq!(
        program.read_stored_instance_field(&argument_exception, &message_field),
        Some(DexValue::String(std::rc::Rc::from("bad argument")))
    );
    assert_eq!(
        program.read_stored_instance_field(&missing_element, &message_field),
        Some(DexValue::String(std::rc::Rc::from("missing")))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &error_with_cause,
            Some(&test_invocation(vec![
                fatal_error.clone(),
                DexValue::String(std::rc::Rc::from("fatal")),
                argument_exception.clone(),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&fatal_error, &message_field),
        Some(DexValue::String(std::rc::Rc::from("fatal")))
    );
    assert_eq!(
        program.read_stored_instance_field(&fatal_error, &cause_field),
        Some(argument_exception.clone())
    );
    assert_eq!(
        program.invoke_builtin_method(
            &init_cause,
            Some(&test_invocation(vec![
                exception.clone(),
                argument_exception.clone(),
            ]))
        ),
        Some(DexExecutionResult::Value(exception.clone()))
    );
    assert_eq!(
        program.read_stored_instance_field(&exception, &cause_field),
        Some(argument_exception)
    );
    let Some(DexExecutionResult::Value(empty_suppressed_array)) = program.invoke_builtin_method(
        &get_suppressed,
        Some(&test_invocation(vec![exception.clone()])),
    ) else {
        panic!("getSuppressed should return an empty Throwable array before addSuppressed");
    };
    let empty_suppressed_array_key = map_identity_key(&empty_suppressed_array);
    assert_eq!(
        program
            .runtime
            .borrow()
            .array_entries
            .get(&empty_suppressed_array_key),
        Some(&Vec::new())
    );
    assert_eq!(
        program.invoke_builtin_method(
            &add_suppressed,
            Some(&test_invocation(vec![
                fatal_error.clone(),
                exception.clone()
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    let Some(suppressed_array) =
        program.read_stored_instance_field(&fatal_error, &suppressed_field)
    else {
        panic!("addSuppressed should store a suppressed throwable array");
    };
    let suppressed_array_key = map_identity_key(&suppressed_array);
    assert_eq!(
        program
            .runtime
            .borrow()
            .array_entries
            .get(&suppressed_array_key),
        Some(&vec![exception.clone()])
    );
    let Some(DexExecutionResult::Value(returned_suppressed_array)) =
        program.invoke_builtin_method(&get_suppressed, Some(&test_invocation(vec![fatal_error])))
    else {
        panic!("getSuppressed should return a Throwable array after addSuppressed");
    };
    let returned_suppressed_array_key = map_identity_key(&returned_suppressed_array);
    assert_eq!(
        program
            .runtime
            .borrow()
            .array_entries
            .get(&returned_suppressed_array_key),
        Some(&vec![exception])
    );
}

#[test]
fn builtin_system_current_time_millis_returns_boot_wall_clock_plus_uptime() {
    let program = DexProgram::new(Vec::new());
    {
        let mut runtime = program.runtime.borrow_mut();
        runtime.boot_wall_clock_millis = 1_000_000;
        runtime.main_thread_uptime_millis = 123;
    }
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/System;".to_owned(),
        name: "currentTimeMillis".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "J".to_owned(),
            return_type: "J".to_owned(),
            parameters: Vec::new(),
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&method, None),
        Some(DexExecutionResult::Value(DexValue::Long(1_000_123)))
    );
}

#[test]
fn libcore_system_get_security_manager_returns_null() {
    // cite: libcore System.java#getSecurityManager
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/System;".to_owned(),
        name: "getSecurityManager".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/SecurityManager;".to_owned(),
            parameters: Vec::new(),
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&method, None),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
}

#[test]
fn builtin_system_nano_time_returns_monotonic_runtime_time() {
    let program = DexProgram::new(Vec::new());
    program.runtime.borrow_mut().main_thread_uptime_millis = 123;
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/System;".to_owned(),
        name: "nanoTime".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "J".to_owned(),
            return_type: "J".to_owned(),
            parameters: Vec::new(),
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&method, None),
        Some(DexExecutionResult::Value(DexValue::Long(123_000_000)))
    );
}

#[test]
fn builtin_instant_now_uses_runtime_wall_clock() {
    let program = DexProgram::new(Vec::new());
    {
        let mut runtime = program.runtime.borrow_mut();
        runtime.boot_wall_clock_millis = 1_234_000;
        runtime.main_thread_uptime_millis = 567;
    }
    let now = DexMethodId {
        class_descriptor: "Ljava/time/Instant;".to_owned(),
        name: "now".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/time/Instant;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let to_epoch_milli = instant_to_epoch_milli_method();
    let get_epoch_second = instant_get_epoch_second_method();
    let get_nano = instant_get_nano_method();

    let Some(DexExecutionResult::Value(instant)) = program.invoke_builtin_method(&now, None) else {
        panic!("Instant.now should return an Instant");
    };
    assert_eq!(
        program.invoke_builtin_method(
            &to_epoch_milli,
            Some(&test_invocation(vec![instant.clone()]))
        ),
        Some(DexExecutionResult::Value(DexValue::Long(1_234_567)))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_epoch_second,
            Some(&test_invocation(vec![instant.clone()]))
        ),
        Some(DexExecutionResult::Value(DexValue::Long(1234)))
    );
    assert_eq!(
        program.invoke_builtin_method(&get_nano, Some(&test_invocation(vec![instant]))),
        Some(DexExecutionResult::Int(567_000_000))
    );
}

#[test]
fn builtin_instant_of_epoch_milli_round_trips_negative_millis() {
    let program = DexProgram::new(Vec::new());
    let of_epoch_milli = DexMethodId {
        class_descriptor: "Ljava/time/Instant;".to_owned(),
        name: "ofEpochMilli".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LJ".to_owned(),
            return_type: "Ljava/time/Instant;".to_owned(),
            parameters: vec!["J".to_owned()],
        },
    };
    let to_epoch_milli = instant_to_epoch_milli_method();
    let get_epoch_second = instant_get_epoch_second_method();
    let get_nano = instant_get_nano_method();

    let Some(DexExecutionResult::Value(instant)) = program.invoke_builtin_method(
        &of_epoch_milli,
        Some(&test_invocation(vec![
            DexValue::Long(-1),
            DexValue::Long(-1),
        ])),
    ) else {
        panic!("Instant.ofEpochMilli should return an Instant");
    };
    assert_eq!(
        program.invoke_builtin_method(
            &to_epoch_milli,
            Some(&test_invocation(vec![instant.clone()]))
        ),
        Some(DexExecutionResult::Value(DexValue::Long(-1)))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_epoch_second,
            Some(&test_invocation(vec![instant.clone()]))
        ),
        Some(DexExecutionResult::Value(DexValue::Long(-1)))
    );
    assert_eq!(
        program.invoke_builtin_method(&get_nano, Some(&test_invocation(vec![instant]))),
        Some(DexExecutionResult::Int(999_000_000))
    );
}

#[test]
fn builtin_system_get_property_returns_known_or_default_values() {
    let program = DexProgram::new(Vec::new());
    let get_property = DexMethodId {
        class_descriptor: "Ljava/lang/System;".to_owned(),
        name: "getProperty".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned()],
        },
    };
    let get_property_with_default = DexMethodId {
        class_descriptor: "Ljava/lang/System;".to_owned(),
        name: "getProperty".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LLL".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: vec![
                "Ljava/lang/String;".to_owned(),
                "Ljava/lang/String;".to_owned(),
            ],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(
            &get_property,
            Some(&test_invocation(vec![DexValue::String(std::rc::Rc::from(
                "line.separator"
            ))]))
        ),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("\n")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_property,
            Some(&test_invocation(vec![DexValue::String(std::rc::Rc::from(
                "winedroid.unknown"
            ))]))
        ),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_property_with_default,
            Some(&test_invocation(vec![
                DexValue::String(std::rc::Rc::from("winedroid.unknown")),
                DexValue::String(std::rc::Rc::from("fallback")),
            ]))
        ),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("fallback")
        )))
    );
}

#[test]
fn builtin_system_arraycopy_copies_array_entries_with_overlap() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/System;".to_owned(),
        name: "arraycopy".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VLILII".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "Ljava/lang/Object;".to_owned(),
                "I".to_owned(),
                "Ljava/lang/Object;".to_owned(),
                "I".to_owned(),
                "I".to_owned(),
            ],
        },
    };
    let source = program.allocate_filled_array(
        "[Ljava/lang/Object;".to_owned(),
        vec![
            DexValue::String(std::rc::Rc::from("a")),
            DexValue::String(std::rc::Rc::from("b")),
            DexValue::String(std::rc::Rc::from("c")),
        ],
    );
    let target = program.allocate_filled_array(
        "[Ljava/lang/Object;".to_owned(),
        vec![DexValue::Null, DexValue::Null, DexValue::Null],
    );
    let overlap = program.allocate_filled_array(
        "[Ljava/lang/Object;".to_owned(),
        vec![
            DexValue::String(std::rc::Rc::from("a")),
            DexValue::String(std::rc::Rc::from("b")),
            DexValue::String(std::rc::Rc::from("c")),
            DexValue::String(std::rc::Rc::from("d")),
        ],
    );

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                source,
                DexValue::Int(1),
                target.clone(),
                DexValue::Int(0),
                DexValue::Int(2),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .array_entries
            .get(&map_identity_key(&target)),
        Some(&vec![
            DexValue::String(std::rc::Rc::from("b")),
            DexValue::String(std::rc::Rc::from("c")),
            DexValue::Null,
        ])
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                overlap.clone(),
                DexValue::Int(0),
                overlap.clone(),
                DexValue::Int(1),
                DexValue::Int(3),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .array_entries
            .get(&map_identity_key(&overlap)),
        Some(&vec![
            DexValue::String(std::rc::Rc::from("a")),
            DexValue::String(std::rc::Rc::from("a")),
            DexValue::String(std::rc::Rc::from("b")),
            DexValue::String(std::rc::Rc::from("c")),
        ])
    );
}

#[test]
fn builtin_system_identity_hash_code_returns_stable_identity_hash() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/System;".to_owned(),
        name: "identityHashCode".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "IL".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let object = DexValue::Object(DexObjectRef {
        id: 23,
        class_descriptor: std::rc::Rc::from("Lcom/example/ComposeNode;"),
    });

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![object]))),
        Some(DexExecutionResult::Int(stable_hash_i32("object:23")))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![DexValue::Reference(
                std::rc::Rc::from("Ljava/lang/Thread;:main")
            )]))
        ),
        Some(DexExecutionResult::Int(stable_hash_i32(
            "reference:Ljava/lang/Thread;:main"
        )))
    );
}

#[test]
fn builtin_print_stream_println_returns_void() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/io/PrintStream;".to_owned(),
        name: "println".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let invocation = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from("Ljava/io/PrintStream;:System.out")),
        DexValue::String(std::rc::Rc::from("message")),
    ]);
    let system_out = DexFieldId {
        class_descriptor: "Ljava/lang/System;".to_owned(),
        type_descriptor: "Ljava/io/PrintStream;".to_owned(),
        name: "out".to_owned(),
    };

    assert_eq!(
        bootstrap_static_field(&system_out),
        Some(DexValue::Reference(std::rc::Rc::from(
            "Ljava/io/PrintStream;:System.out"
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Void)
    );
}

#[test]
fn builtin_print_stream_printf_returns_receiver_per_libcore_print_stream_java() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/io/PrintStream;".to_owned(),
        name: "printf".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LLL".to_owned(),
            return_type: "Ljava/io/PrintStream;".to_owned(),
            parameters: vec![
                "Ljava/lang/String;".to_owned(),
                "[Ljava/lang/Object;".to_owned(),
            ],
        },
    };
    let receiver = DexValue::Reference(std::rc::Rc::from("Ljava/io/PrintStream;:System.out"));
    let arguments = program.allocate_filled_array(
        "[Ljava/lang/Object;".to_owned(),
        vec![DexValue::Int(7)],
    );
    let with_arguments = test_invocation(vec![
        receiver.clone(),
        DexValue::String(std::rc::Rc::from("value=%d")),
        arguments,
    ]);
    let null_arguments = test_invocation(vec![
        receiver.clone(),
        DexValue::String(std::rc::Rc::from("plain")),
        DexValue::Null,
    ]);
    let missing_argument = test_invocation(vec![
        receiver.clone(),
        DexValue::String(std::rc::Rc::from("%s")),
        DexValue::Null,
    ]);
    let null_receiver = test_invocation(vec![
        DexValue::Null,
        DexValue::String(std::rc::Rc::from("plain")),
        DexValue::Null,
    ]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&with_arguments)),
        Some(DexExecutionResult::Value(receiver.clone()))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&null_arguments)),
        Some(DexExecutionResult::Value(receiver))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&missing_argument)),
        None
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&null_receiver)),
        None
    );
}

#[test]
fn bootstraps_window_insets_consumed_static_field() {
    let field = DexFieldId {
        class_descriptor: "Landroid/view/WindowInsets;".to_owned(),
        type_descriptor: "Landroid/view/WindowInsets;".to_owned(),
        name: "CONSUMED".to_owned(),
    };

    assert_eq!(
        bootstrap_static_field(&field),
        Some(DexValue::Reference(std::rc::Rc::from(
            "Landroid/view/WindowInsets;.CONSUMED"
        )))
    );
}

#[test]
fn builtin_runtime_get_runtime_returns_bootstrap_reference() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Runtime;".to_owned(),
        name: "getRuntime".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/Runtime;".to_owned(),
            parameters: Vec::new(),
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&method, None),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Ljava/lang/Runtime;.bootstrap")
        )))
    );
}

#[test]
fn builtin_runtime_available_processors_returns_positive_count() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Runtime;".to_owned(),
        name: "availableProcessors".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };

    let Some(DexExecutionResult::Int(value)) = program.invoke_builtin_method(&method, None) else {
        panic!("availableProcessors should return an int");
    };

    assert!(value > 0);
}

#[test]
fn builtin_math_min_and_max_return_integer_extrema() {
    let program = DexProgram::new(Vec::new());
    let min = DexMethodId {
        class_descriptor: "Ljava/lang/Math;".to_owned(),
        name: "min".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "III".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["I".to_owned(), "I".to_owned()],
        },
    };
    let max = DexMethodId {
        class_descriptor: "Ljava/lang/Math;".to_owned(),
        name: "max".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "III".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["I".to_owned(), "I".to_owned()],
        },
    };
    let min_long = DexMethodId {
        class_descriptor: "Ljava/lang/Math;".to_owned(),
        name: "min".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "JJJ".to_owned(),
            return_type: "J".to_owned(),
            parameters: vec!["J".to_owned(), "J".to_owned()],
        },
    };
    let max_long = DexMethodId {
        class_descriptor: "Ljava/lang/Math;".to_owned(),
        name: "max".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "JJJ".to_owned(),
            return_type: "J".to_owned(),
            parameters: vec!["J".to_owned(), "J".to_owned()],
        },
    };
    let abs = DexMethodId {
        class_descriptor: "Ljava/lang/Math;".to_owned(),
        name: "abs".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "DD".to_owned(),
            return_type: "D".to_owned(),
            parameters: vec!["D".to_owned()],
        },
    };
    let log = DexMethodId {
        class_descriptor: "Ljava/lang/Math;".to_owned(),
        name: "log".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "DD".to_owned(),
            return_type: "D".to_owned(),
            parameters: vec!["D".to_owned()],
        },
    };
    let exp = DexMethodId {
        class_descriptor: "Ljava/lang/Math;".to_owned(),
        name: "exp".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "DD".to_owned(),
            return_type: "D".to_owned(),
            parameters: vec!["D".to_owned()],
        },
    };
    let log10 = DexMethodId {
        class_descriptor: "Ljava/lang/Math;".to_owned(),
        name: "log10".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "DD".to_owned(),
            return_type: "D".to_owned(),
            parameters: vec!["D".to_owned()],
        },
    };
    let ulp = DexMethodId {
        class_descriptor: "Ljava/lang/Math;".to_owned(),
        name: "ulp".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "DD".to_owned(),
            return_type: "D".to_owned(),
            parameters: vec!["D".to_owned()],
        },
    };
    let sqrt = DexMethodId {
        class_descriptor: "Ljava/lang/Math;".to_owned(),
        name: "sqrt".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "DD".to_owned(),
            return_type: "D".to_owned(),
            parameters: vec!["D".to_owned()],
        },
    };
    let pow = DexMethodId {
        class_descriptor: "Ljava/lang/Math;".to_owned(),
        name: "pow".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "DDD".to_owned(),
            return_type: "D".to_owned(),
            parameters: vec!["D".to_owned(), "D".to_owned()],
        },
    };
    let hypot = DexMethodId {
        class_descriptor: "Ljava/lang/Math;".to_owned(),
        name: "hypot".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "DDD".to_owned(),
            return_type: "D".to_owned(),
            parameters: vec!["D".to_owned(), "D".to_owned()],
        },
    };
    let to_radians = DexMethodId {
        class_descriptor: "Ljava/lang/Math;".to_owned(),
        name: "toRadians".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "DD".to_owned(),
            return_type: "D".to_owned(),
            parameters: vec!["D".to_owned()],
        },
    };
    let sin = DexMethodId {
        class_descriptor: "Ljava/lang/Math;".to_owned(),
        name: "sin".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "DD".to_owned(),
            return_type: "D".to_owned(),
            parameters: vec!["D".to_owned()],
        },
    };
    let asin = DexMethodId {
        class_descriptor: "Ljava/lang/Math;".to_owned(),
        name: "asin".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "DD".to_owned(),
            return_type: "D".to_owned(),
            parameters: vec!["D".to_owned()],
        },
    };
    let to_degrees = DexMethodId {
        class_descriptor: "Ljava/lang/Math;".to_owned(),
        name: "toDegrees".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "DD".to_owned(),
            return_type: "D".to_owned(),
            parameters: vec!["D".to_owned()],
        },
    };
    let cos = DexMethodId {
        class_descriptor: "Ljava/lang/Math;".to_owned(),
        name: "cos".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "DD".to_owned(),
            return_type: "D".to_owned(),
            parameters: vec!["D".to_owned()],
        },
    };
    let acos = DexMethodId {
        class_descriptor: "Ljava/lang/Math;".to_owned(),
        name: "acos".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "DD".to_owned(),
            return_type: "D".to_owned(),
            parameters: vec!["D".to_owned()],
        },
    };
    let tan = DexMethodId {
        class_descriptor: "Ljava/lang/Math;".to_owned(),
        name: "tan".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "DD".to_owned(),
            return_type: "D".to_owned(),
            parameters: vec!["D".to_owned()],
        },
    };
    let atan = DexMethodId {
        class_descriptor: "Ljava/lang/Math;".to_owned(),
        name: "atan".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "DD".to_owned(),
            return_type: "D".to_owned(),
            parameters: vec!["D".to_owned()],
        },
    };
    let invocation = test_invocation(vec![DexValue::Int(7), DexValue::Int(3)]);
    let long_invocation = test_invocation(vec![
        DexValue::Long(-4),
        DexValue::Long(-4),
        DexValue::Long(9),
        DexValue::Long(9),
    ]);
    let double_bits = 10.0_f64.to_bits() as i64;
    let double_invocation = test_invocation(vec![
        DexValue::Long(double_bits),
        DexValue::Long(double_bits),
    ]);
    let expected_log_bits = 10.0_f64.ln().to_bits() as i64;
    let expected_exp_bits = 10.0_f64.exp().to_bits() as i64;
    let expected_log10_bits = 10.0_f64.log10().to_bits() as i64;
    let negative_bits = (-7.5_f64).to_bits() as i64;
    let negative_invocation = test_invocation(vec![
        DexValue::Long(negative_bits),
        DexValue::Long(negative_bits),
    ]);
    let pow_left = 2.0_f64.to_bits() as i64;
    let pow_right = 3.0_f64.to_bits() as i64;
    let pow_invocation = test_invocation(vec![
        DexValue::Long(pow_left),
        DexValue::Long(pow_left),
        DexValue::Long(pow_right),
        DexValue::Long(pow_right),
    ]);
    let expected_pow_bits = 8.0_f64.to_bits() as i64;
    let angle_bits = 30.0_f64.to_bits() as i64;
    let angle_invocation =
        test_invocation(vec![DexValue::Long(angle_bits), DexValue::Long(angle_bits)]);
    let radians = 30.0_f64.to_radians();
    let half_bits = 0.5_f64.to_bits() as i64;
    let half_invocation =
        test_invocation(vec![DexValue::Long(half_bits), DexValue::Long(half_bits)]);

    assert_eq!(
        program.invoke_builtin_method(&min, Some(&invocation)),
        Some(DexExecutionResult::Int(3))
    );
    assert_eq!(
        program.invoke_builtin_method(&max, Some(&invocation)),
        Some(DexExecutionResult::Int(7))
    );
    assert_eq!(
        program.invoke_builtin_method(&min_long, Some(&long_invocation)),
        Some(DexExecutionResult::Value(DexValue::Long(-4)))
    );
    assert_eq!(
        program.invoke_builtin_method(&max_long, Some(&long_invocation)),
        Some(DexExecutionResult::Value(DexValue::Long(9)))
    );
    assert_eq!(
        program.invoke_builtin_method(&abs, Some(&negative_invocation)),
        Some(DexExecutionResult::Value(DexValue::Long(
            7.5_f64.to_bits() as i64
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&log, Some(&double_invocation)),
        Some(DexExecutionResult::Value(DexValue::Long(expected_log_bits)))
    );
    assert_eq!(
        program.invoke_builtin_method(&exp, Some(&double_invocation)),
        Some(DexExecutionResult::Value(DexValue::Long(expected_exp_bits)))
    );
    assert_eq!(
        program.invoke_builtin_method(&log10, Some(&double_invocation)),
        Some(DexExecutionResult::Value(DexValue::Long(
            expected_log10_bits
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &ulp,
            Some(&test_invocation(vec![
                DexValue::Long(1.0_f64.to_bits() as i64),
                DexValue::Long(1.0_f64.to_bits() as i64),
            ]))
        ),
        Some(DexExecutionResult::Value(DexValue::Long(
            f64::EPSILON.to_bits() as i64
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &ulp,
            Some(&test_invocation(vec![
                DexValue::Long(0.0_f64.to_bits() as i64),
                DexValue::Long(0.0_f64.to_bits() as i64),
            ]))
        ),
        Some(DexExecutionResult::Value(DexValue::Long(1)))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &sqrt,
            Some(&test_invocation(vec![
                DexValue::Long(9.0_f64.to_bits() as i64),
                DexValue::Long(9.0_f64.to_bits() as i64),
            ]))
        ),
        Some(DexExecutionResult::Value(DexValue::Long(
            3.0_f64.to_bits() as i64
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&pow, Some(&pow_invocation)),
        Some(DexExecutionResult::Value(DexValue::Long(expected_pow_bits)))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &hypot,
            Some(&test_invocation(vec![
                DexValue::Long(3.0_f64.to_bits() as i64),
                DexValue::Long(3.0_f64.to_bits() as i64),
                DexValue::Long(4.0_f64.to_bits() as i64),
                DexValue::Long(4.0_f64.to_bits() as i64),
            ]))
        ),
        Some(DexExecutionResult::Value(DexValue::Long(
            5.0_f64.to_bits() as i64
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&to_radians, Some(&angle_invocation)),
        Some(DexExecutionResult::Value(DexValue::Long(
            radians.to_bits() as i64
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &to_degrees,
            Some(&test_invocation(vec![
                DexValue::Long(radians.to_bits() as i64),
                DexValue::Long(radians.to_bits() as i64),
            ]))
        ),
        Some(DexExecutionResult::Value(DexValue::Long(
            radians.to_degrees().to_bits() as i64
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &sin,
            Some(&test_invocation(vec![
                DexValue::Long(radians.to_bits() as i64),
                DexValue::Long(radians.to_bits() as i64),
            ]))
        ),
        Some(DexExecutionResult::Value(DexValue::Long(
            radians.sin().to_bits() as i64
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&asin, Some(&half_invocation)),
        Some(DexExecutionResult::Value(DexValue::Long(
            0.5_f64.asin().to_bits() as i64
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &cos,
            Some(&test_invocation(vec![
                DexValue::Long(radians.to_bits() as i64),
                DexValue::Long(radians.to_bits() as i64),
            ]))
        ),
        Some(DexExecutionResult::Value(DexValue::Long(
            radians.cos().to_bits() as i64
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&acos, Some(&half_invocation)),
        Some(DexExecutionResult::Value(DexValue::Long(
            0.5_f64.acos().to_bits() as i64
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &tan,
            Some(&test_invocation(vec![
                DexValue::Long(radians.to_bits() as i64),
                DexValue::Long(radians.to_bits() as i64),
            ]))
        ),
        Some(DexExecutionResult::Value(DexValue::Long(
            radians.tan().to_bits() as i64
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&atan, Some(&half_invocation)),
        Some(DexExecutionResult::Value(DexValue::Long(
            0.5_f64.atan().to_bits() as i64
        )))
    );
}

#[test]
fn builtin_math_ceil_double_matches_java_behavior() {
    let program = DexProgram::new(Vec::new());
    let ceil = DexMethodId {
        class_descriptor: "Ljava/lang/Math;".to_owned(),
        name: "ceil".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "DD".to_owned(),
            return_type: "D".to_owned(),
            parameters: vec!["D".to_owned()],
        },
    };

    for (input, expected) in [(1.2_f64, 2.0_f64), (-1.2_f64, -1.0_f64)] {
        let bits = input.to_bits() as i64;
        assert_eq!(
            program.invoke_builtin_method(
                &ceil,
                Some(&test_invocation(vec![
                    DexValue::Long(bits),
                    DexValue::Long(bits),
                ]))
            ),
            Some(DexExecutionResult::Value(DexValue::Long(
                expected.to_bits() as i64
            )))
        );
    }
}

#[test]
fn builtin_math_floor_double_matches_java_behavior() {
    let program = DexProgram::new(Vec::new());
    let floor = DexMethodId {
        class_descriptor: "Ljava/lang/Math;".to_owned(),
        name: "floor".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "DD".to_owned(),
            return_type: "D".to_owned(),
            parameters: vec!["D".to_owned()],
        },
    };

    for (input, expected) in [(1.2_f64, 1.0_f64), (-1.2_f64, -2.0_f64)] {
        let bits = input.to_bits() as i64;
        assert_eq!(
            program.invoke_builtin_method(
                &floor,
                Some(&test_invocation(vec![
                    DexValue::Long(bits),
                    DexValue::Long(bits),
                ]))
            ),
            Some(DexExecutionResult::Value(DexValue::Long(
                expected.to_bits() as i64
            )))
        );
    }
}

#[test]
fn builtin_math_rint_double_matches_java_behavior() {
    let program = DexProgram::new(Vec::new());
    let rint = DexMethodId {
        class_descriptor: "Ljava/lang/Math;".to_owned(),
        name: "rint".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "DD".to_owned(),
            return_type: "D".to_owned(),
            parameters: vec!["D".to_owned()],
        },
    };

    for (input, expected) in [
        (1.2_f64, 1.0_f64),
        (1.5_f64, 2.0_f64),
        (2.5_f64, 2.0_f64),
        (-1.5_f64, -2.0_f64),
        (-0.2_f64, -0.0_f64),
    ] {
        let bits = input.to_bits() as i64;
        assert_eq!(
            program.invoke_builtin_method(
                &rint,
                Some(&test_invocation(vec![
                    DexValue::Long(bits),
                    DexValue::Long(bits),
                ]))
            ),
            Some(DexExecutionResult::Value(DexValue::Long(
                expected.to_bits() as i64
            )))
        );
    }
}

#[test]
fn builtin_math_min_and_max_float_match_java_behavior() {
    let program = DexProgram::new(Vec::new());
    let min = DexMethodId {
        class_descriptor: "Ljava/lang/Math;".to_owned(),
        name: "min".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "FFF".to_owned(),
            return_type: "F".to_owned(),
            parameters: vec!["F".to_owned(), "F".to_owned()],
        },
    };
    let max = DexMethodId {
        class_descriptor: "Ljava/lang/Math;".to_owned(),
        name: "max".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "FFF".to_owned(),
            return_type: "F".to_owned(),
            parameters: vec!["F".to_owned(), "F".to_owned()],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(
            &min,
            Some(&test_invocation(vec![
                DexValue::Float(0.5),
                DexValue::Int(1.0_f32.to_bits() as i32)
            ]))
        ),
        Some(DexExecutionResult::Value(DexValue::Float(0.5)))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &max,
            Some(&test_invocation(vec![
                DexValue::Float(0.5),
                DexValue::Int(1.0_f32.to_bits() as i32)
            ]))
        ),
        Some(DexExecutionResult::Value(DexValue::Float(1.0)))
    );

    let Some(DexExecutionResult::Value(DexValue::Float(min_zero))) = program.invoke_builtin_method(
        &min,
        Some(&test_invocation(vec![
            DexValue::Float(0.0),
            DexValue::Float(-0.0),
        ])),
    ) else {
        panic!("Math.min(0.0f, -0.0f) did not return a float");
    };
    let Some(DexExecutionResult::Value(DexValue::Float(max_zero))) = program.invoke_builtin_method(
        &max,
        Some(&test_invocation(vec![
            DexValue::Float(0.0),
            DexValue::Float(-0.0),
        ])),
    ) else {
        panic!("Math.max(0.0f, -0.0f) did not return a float");
    };
    let Some(DexExecutionResult::Value(DexValue::Float(nan_value))) = program
        .invoke_builtin_method(
            &min,
            Some(&test_invocation(vec![
                DexValue::Float(f32::NAN),
                DexValue::Float(1.0),
            ])),
        )
    else {
        panic!("Math.min(NaN, 1.0f) did not return a float");
    };

    assert_eq!(min_zero.to_bits(), (-0.0_f32).to_bits());
    assert_eq!(max_zero.to_bits(), 0.0_f32.to_bits());
    assert!(nan_value.is_nan());
}

#[test]
fn builtin_math_signum_float_matches_java_behavior() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Math;".to_owned(),
        name: "signum".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "FF".to_owned(),
            return_type: "F".to_owned(),
            parameters: vec!["F".to_owned()],
        },
    };

    for (input, expected) in [
        (-2.5_f32, -1.0_f32),
        (-0.0_f32, -0.0_f32),
        (0.0_f32, 0.0_f32),
        (2.5_f32, 1.0_f32),
        (f32::INFINITY, 1.0_f32),
        (f32::NEG_INFINITY, -1.0_f32),
    ] {
        let Some(DexExecutionResult::Value(DexValue::Float(value))) = program
            .invoke_builtin_method(
                &method,
                Some(&test_invocation(vec![DexValue::Float(input)])),
            )
        else {
            panic!("Math.signum(float) did not return a float");
        };

        assert_eq!(value.to_bits(), expected.to_bits());
    }

    let Some(DexExecutionResult::Value(DexValue::Float(value))) = program.invoke_builtin_method(
        &method,
        Some(&test_invocation(vec![DexValue::Int(
            f32::NAN.to_bits() as i32
        )])),
    ) else {
        panic!("Math.signum(NaN) did not return a float");
    };

    assert!(value.is_nan());
}

#[test]
fn builtin_math_max_double_matches_java_behavior() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Math;".to_owned(),
        name: "max".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "DDD".to_owned(),
            return_type: "D".to_owned(),
            parameters: vec!["D".to_owned(), "D".to_owned()],
        },
    };
    let half_bits = 0.5_f64.to_bits() as i64;
    let one_bits = 1.0_f64.to_bits() as i64;

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::Long(half_bits),
                DexValue::Long(half_bits),
                DexValue::Long(one_bits),
                DexValue::Long(one_bits),
            ]))
        ),
        Some(DexExecutionResult::Value(DexValue::Long(one_bits)))
    );

    let positive_zero_bits = 0.0_f64.to_bits() as i64;
    let negative_zero_bits = (-0.0_f64).to_bits() as i64;
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::Long(negative_zero_bits),
                DexValue::Long(negative_zero_bits),
                DexValue::Long(positive_zero_bits),
                DexValue::Long(positive_zero_bits),
            ]))
        ),
        Some(DexExecutionResult::Value(DexValue::Long(
            positive_zero_bits
        )))
    );

    let nan_bits = f64::NAN.to_bits() as i64;
    let Some(DexExecutionResult::Value(DexValue::Long(result_bits))) = program
        .invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::Long(nan_bits),
                DexValue::Long(nan_bits),
                DexValue::Long(one_bits),
                DexValue::Long(one_bits),
            ])),
        )
    else {
        panic!("Math.max(NaN, 1.0d) did not return double bits");
    };
    assert!(f64::from_bits(result_bits as u64).is_nan());
}

#[test]
fn builtin_math_round_float_matches_java_behavior() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Math;".to_owned(),
        name: "round".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "IF".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["F".to_owned()],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![DexValue::Float(1.5)]))),
        Some(DexExecutionResult::Int(2))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![DexValue::Float(-1.5)]))),
        Some(DexExecutionResult::Int(-1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![DexValue::Float(f32::NAN)]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![DexValue::Float(f32::INFINITY)]))
        ),
        Some(DexExecutionResult::Int(i32::MAX))
    );
}

#[test]
fn builtin_math_round_double_matches_java_behavior() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Math;".to_owned(),
        name: "round".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "JD".to_owned(),
            return_type: "J".to_owned(),
            parameters: vec!["D".to_owned()],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::Long(1.5_f64.to_bits() as i64),
                DexValue::Long(1.5_f64.to_bits() as i64),
            ]))
        ),
        Some(DexExecutionResult::Value(DexValue::Long(2)))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::Long((-1.5_f64).to_bits() as i64),
                DexValue::Long((-1.5_f64).to_bits() as i64),
            ]))
        ),
        Some(DexExecutionResult::Value(DexValue::Long(-1)))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::Long(f64::NAN.to_bits() as i64),
                DexValue::Long(f64::NAN.to_bits() as i64),
            ]))
        ),
        Some(DexExecutionResult::Value(DexValue::Long(0)))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::Long(f64::INFINITY.to_bits() as i64),
                DexValue::Long(f64::INFINITY.to_bits() as i64),
            ]))
        ),
        Some(DexExecutionResult::Value(DexValue::Long(i64::MAX)))
    );
}

#[test]
fn builtin_math_abs_float_matches_java_behavior() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Math;".to_owned(),
        name: "abs".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "FF".to_owned(),
            return_type: "F".to_owned(),
            parameters: vec!["F".to_owned()],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![DexValue::Float(-1.5)]))),
        Some(DexExecutionResult::Value(DexValue::Float(1.5)))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![DexValue::Int(
                (-0.25_f32).to_bits() as i32
            )]))
        ),
        Some(DexExecutionResult::Value(DexValue::Float(0.25)))
    );
}

#[test]
fn builtin_math_abs_int_matches_java_overflow_behavior() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Math;".to_owned(),
        name: "abs".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "II".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![DexValue::Int(-7)]))),
        Some(DexExecutionResult::Int(7))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![DexValue::Int(i32::MIN)]))
        ),
        Some(DexExecutionResult::Int(i32::MIN))
    );
}

#[test]
fn builtin_integer_number_of_leading_zeros_matches_java_behavior() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Integer;".to_owned(),
        name: "numberOfLeadingZeros".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "II".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![DexValue::Int(0)]))),
        Some(DexExecutionResult::Int(32))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![DexValue::Int(1)]))),
        Some(DexExecutionResult::Int(31))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![DexValue::Int(6)]))),
        Some(DexExecutionResult::Int(29))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![DexValue::Int(-1)]))),
        Some(DexExecutionResult::Int(0))
    );
}

#[test]
fn builtin_integer_bit_count_matches_java_behavior() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Integer;".to_owned(),
        name: "bitCount".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "II".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![DexValue::Int(0)]))),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![DexValue::Int(7)]))),
        Some(DexExecutionResult::Int(3))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![DexValue::Int(-1)]))),
        Some(DexExecutionResult::Int(32))
    );
}

#[test]
fn builtin_integer_lowest_one_bit_matches_java_behavior() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Integer;".to_owned(),
        name: "lowestOneBit".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "II".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![DexValue::Int(0)]))),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![DexValue::Int(12)]))),
        Some(DexExecutionResult::Int(4))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![DexValue::Int(i32::MIN)]))
        ),
        Some(DexExecutionResult::Int(i32::MIN))
    );
}

#[test]
fn builtin_integer_highest_one_bit_matches_java_behavior() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Integer;".to_owned(),
        name: "highestOneBit".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "II".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![DexValue::Int(0)]))),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![DexValue::Int(24)]))),
        Some(DexExecutionResult::Int(16))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![DexValue::Int(i32::MIN)]))
        ),
        Some(DexExecutionResult::Int(i32::MIN))
    );
}

#[test]
fn builtin_integer_hash_code_returns_the_integer_value() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Integer;".to_owned(),
        name: "hashCode".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "II".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![DexValue::Int(0)]))),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![DexValue::Int(37)]))),
        Some(DexExecutionResult::Int(37))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![DexValue::Int(-4)]))),
        Some(DexExecutionResult::Int(-4))
    );
}

#[test]
fn builtin_integer_signum_matches_java_behavior() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Integer;".to_owned(),
        name: "signum".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "II".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![DexValue::Int(-42)]))),
        Some(DexExecutionResult::Int(-1))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![DexValue::Int(0)]))),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![DexValue::Int(42)]))),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![DexValue::Int(i32::MIN)]))
        ),
        Some(DexExecutionResult::Int(-1))
    );
}

#[test]
fn builtin_long_number_of_trailing_zeros_matches_java_behavior() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Long;".to_owned(),
        name: "numberOfTrailingZeros".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "IJ".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["J".to_owned()],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![DexValue::Long(0), DexValue::Long(0)]))
        ),
        Some(DexExecutionResult::Int(64))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![DexValue::Long(1), DexValue::Long(1)]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![DexValue::Long(8), DexValue::Long(8)]))
        ),
        Some(DexExecutionResult::Int(3))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::Long(-8),
                DexValue::Long(-8)
            ]))
        ),
        Some(DexExecutionResult::Int(3))
    );
}

#[test]
fn builtin_long_number_of_leading_zeros_matches_java_behavior() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Long;".to_owned(),
        name: "numberOfLeadingZeros".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "IJ".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["J".to_owned()],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![DexValue::Long(0), DexValue::Long(0)]))
        ),
        Some(DexExecutionResult::Int(64))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![DexValue::Long(1), DexValue::Long(1)]))
        ),
        Some(DexExecutionResult::Int(63))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::Long(1_i64 << 62),
                DexValue::Long(1_i64 << 62)
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::Long(-1),
                DexValue::Long(-1)
            ]))
        ),
        Some(DexExecutionResult::Int(0))
    );
}

#[test]
fn builtin_long_compare_unsigned_matches_java_behavior() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Long;".to_owned(),
        name: "compareUnsigned".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "IJJ".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["J".to_owned(), "J".to_owned()],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::Long(-1),
                DexValue::Long(-1),
                DexValue::Long(1),
                DexValue::Long(1),
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::Long(2),
                DexValue::Long(2),
                DexValue::Long(-1),
                DexValue::Long(-1),
            ]))
        ),
        Some(DexExecutionResult::Int(-1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::Long(7),
                DexValue::Long(7),
                DexValue::Long(7),
                DexValue::Long(7),
            ]))
        ),
        Some(DexExecutionResult::Int(0))
    );
}

#[test]
fn builtin_long_hash_code_matches_java_behavior() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Long;".to_owned(),
        name: "hashCode".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "IJ".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["J".to_owned()],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![DexValue::Long(3), DexValue::Long(3)]))
        ),
        Some(DexExecutionResult::Int(3))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::Long(0x0000_0001_0000_0002),
                DexValue::Long(0x0000_0001_0000_0002)
            ]))
        ),
        Some(DexExecutionResult::Int(3))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::Long(-1),
                DexValue::Long(-1)
            ]))
        ),
        Some(DexExecutionResult::Int(0))
    );
}

#[test]
fn builtin_long_signum_matches_java_behavior() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Long;".to_owned(),
        name: "signum".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "IJ".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["J".to_owned()],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::Long(-20),
                DexValue::Long(-20)
            ]))
        ),
        Some(DexExecutionResult::Int(-1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![DexValue::Long(0), DexValue::Long(0)]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::Long(20),
                DexValue::Long(20)
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
}

#[test]
fn builtin_long_rotate_left_matches_java_behavior() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Long;".to_owned(),
        name: "rotateLeft".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "JJI".to_owned(),
            return_type: "J".to_owned(),
            parameters: vec!["J".to_owned(), "I".to_owned()],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::Long(1),
                DexValue::Long(1),
                DexValue::Int(3),
            ]))
        ),
        Some(DexExecutionResult::Value(DexValue::Long(8)))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::Long(8),
                DexValue::Long(8),
                DexValue::Int(-1),
            ]))
        ),
        Some(DexExecutionResult::Value(DexValue::Long(4)))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::Long(0x4000_0000_0000_0000),
                DexValue::Long(0x4000_0000_0000_0000),
                DexValue::Int(1),
            ]))
        ),
        Some(DexExecutionResult::Value(DexValue::Long(i64::MIN)))
    );
}

#[test]
fn builtin_long_rotate_right_matches_java_behavior() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Long;".to_owned(),
        name: "rotateRight".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "JJI".to_owned(),
            return_type: "J".to_owned(),
            parameters: vec!["J".to_owned(), "I".to_owned()],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::Long(8),
                DexValue::Long(8),
                DexValue::Int(3),
            ]))
        ),
        Some(DexExecutionResult::Value(DexValue::Long(1)))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::Long(4),
                DexValue::Long(4),
                DexValue::Int(-1),
            ]))
        ),
        Some(DexExecutionResult::Value(DexValue::Long(8)))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::Long(i64::MIN),
                DexValue::Long(i64::MIN),
                DexValue::Int(1),
            ]))
        ),
        Some(DexExecutionResult::Value(DexValue::Long(
            0x4000_0000_0000_0000
        )))
    );
}

#[test]
fn builtin_float_int_bits_to_float_preserves_ieee_bits() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Float;".to_owned(),
        name: "intBitsToFloat".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "FI".to_owned(),
            return_type: "F".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![DexValue::Int(
                1.5_f32.to_bits() as i32
            )]))
        ),
        Some(DexExecutionResult::Value(DexValue::Float(1.5)))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![DexValue::Int(
                f32::INFINITY.to_bits() as i32
            )]))
        ),
        Some(DexExecutionResult::Value(DexValue::Float(f32::INFINITY)))
    );
}

#[test]
fn builtin_float_to_int_bits_matches_java_behavior() {
    let program = DexProgram::new(Vec::new());
    let raw = DexMethodId {
        class_descriptor: "Ljava/lang/Float;".to_owned(),
        name: "floatToRawIntBits".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "IF".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["F".to_owned()],
        },
    };
    let canonical = DexMethodId {
        class_descriptor: "Ljava/lang/Float;".to_owned(),
        name: "floatToIntBits".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "IF".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["F".to_owned()],
        },
    };
    let custom_nan_bits = 0x7f80_0001_u32 as i32;

    assert_eq!(
        program.invoke_builtin_method(&raw, Some(&test_invocation(vec![DexValue::Float(1.5)]))),
        Some(DexExecutionResult::Int(1.5_f32.to_bits() as i32))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &raw,
            Some(&test_invocation(vec![DexValue::Int(custom_nan_bits)]))
        ),
        Some(DexExecutionResult::Int(custom_nan_bits))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &canonical,
            Some(&test_invocation(vec![DexValue::Int(custom_nan_bits)]))
        ),
        Some(DexExecutionResult::Int(0x7fc0_0000))
    );
}

#[test]
fn builtin_float_hash_code_matches_java_behavior() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Float;".to_owned(),
        name: "hashCode".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "IF".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["F".to_owned()],
        },
    };
    let custom_nan_bits = 0x7f80_0001_u32 as i32;

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![DexValue::Float(1.5)]))),
        Some(DexExecutionResult::Int(1.5_f32.to_bits() as i32))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![DexValue::Int(custom_nan_bits)]))
        ),
        Some(DexExecutionResult::Int(0x7fc0_0000))
    );
}

#[test]
fn builtin_float_compare_matches_java_behavior() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Float;".to_owned(),
        name: "compare".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "IFF".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["F".to_owned(), "F".to_owned()],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::Float(1.0),
                DexValue::Float(2.0)
            ]))
        ),
        Some(DexExecutionResult::Int(-1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::Float(-0.0),
                DexValue::Float(0.0)
            ]))
        ),
        Some(DexExecutionResult::Int(-1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::Float(f32::NAN),
                DexValue::Float(f32::INFINITY)
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::Int(f32::NAN.to_bits() as i32),
                DexValue::Int(f32::NAN.to_bits() as i32)
            ]))
        ),
        Some(DexExecutionResult::Int(0))
    );
}

#[test]
fn builtin_float_is_nan_accepts_float_and_raw_bits() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Float;".to_owned(),
        name: "isNaN".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZF".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["F".to_owned()],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![DexValue::Float(1.0)]))),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![DexValue::Int(
                f32::NAN.to_bits() as i32
            )]))
        ),
        Some(DexExecutionResult::Int(1))
    );
}

#[test]
fn builtin_float_is_infinite_accepts_float_and_raw_bits() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Float;".to_owned(),
        name: "isInfinite".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZF".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["F".to_owned()],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![DexValue::Float(1.0)]))),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![DexValue::Float(f32::INFINITY)]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![DexValue::Int(
                f32::NEG_INFINITY.to_bits() as i32
            )]))
        ),
        Some(DexExecutionResult::Int(1))
    );
}

#[test]
fn builtin_double_is_nan_accepts_raw_wide_bits() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Double;".to_owned(),
        name: "isNaN".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZD".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["D".to_owned()],
        },
    };
    let finite_bits = 1.0_f64.to_bits() as i64;
    let nan_bits = f64::NAN.to_bits() as i64;

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::Long(finite_bits),
                DexValue::Long(finite_bits)
            ]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::Long(nan_bits),
                DexValue::Long(nan_bits)
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
}

#[test]
fn builtin_double_to_string_formats_decimal() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Double;".to_owned(),
        name: "toString".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LD".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: vec!["D".to_owned()],
        },
    };
    let bits = 1.5_f64.to_bits() as i64;

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::Long(bits),
                DexValue::Long(bits)
            ]))
        ),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("1.5")
        )))
    );
}

#[test]
fn builtin_double_to_raw_long_bits_preserves_ieee_payload() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Double;".to_owned(),
        name: "doubleToRawLongBits".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "JD".to_owned(),
            return_type: "J".to_owned(),
            parameters: vec!["D".to_owned()],
        },
    };
    let finite_bits = (-12.5_f64).to_bits() as i64;
    let custom_nan_bits = 0x7ff0_0000_0000_0001_u64 as i64;

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::Long(finite_bits),
                DexValue::Long(finite_bits)
            ]))
        ),
        Some(DexExecutionResult::Value(DexValue::Long(finite_bits)))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::Long(custom_nan_bits),
                DexValue::Long(custom_nan_bits)
            ]))
        ),
        Some(DexExecutionResult::Value(DexValue::Long(custom_nan_bits)))
    );
}

#[test]
fn builtin_decimal_format_symbols_return_root_separators() {
    let program = DexProgram::new(Vec::new());
    let get_instance = DexMethodId {
        class_descriptor: "Ljava/text/DecimalFormatSymbols;".to_owned(),
        name: "getInstance".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/text/DecimalFormatSymbols;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let decimal_separator = DexMethodId {
        class_descriptor: "Ljava/text/DecimalFormatSymbols;".to_owned(),
        name: "getDecimalSeparator".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "C".to_owned(),
            return_type: "C".to_owned(),
            parameters: Vec::new(),
        },
    };
    let grouping_separator = DexMethodId {
        class_descriptor: "Ljava/text/DecimalFormatSymbols;".to_owned(),
        name: "getGroupingSeparator".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "C".to_owned(),
            return_type: "C".to_owned(),
            parameters: Vec::new(),
        },
    };
    let symbols = DexValue::Reference(std::rc::Rc::from("Ljava/text/DecimalFormatSymbols;:root"));
    let symbols_invocation = test_invocation(vec![symbols.clone()]);

    assert_eq!(
        program.invoke_builtin_method(&get_instance, None),
        Some(DexExecutionResult::Value(symbols))
    );
    assert_eq!(
        program.invoke_builtin_method(&decimal_separator, Some(&symbols_invocation)),
        Some(DexExecutionResult::Int('.' as i32))
    );
    assert_eq!(
        program.invoke_builtin_method(&grouping_separator, Some(&symbols_invocation)),
        Some(DexExecutionResult::Int(',' as i32))
    );
}

#[test]
fn builtin_system_clock_methods_return_runtime_uptime() {
    let program = DexProgram::new(Vec::new());
    program.runtime.borrow_mut().main_thread_uptime_millis = 456;
    let elapsed_realtime = DexMethodId {
        class_descriptor: "Landroid/os/SystemClock;".to_owned(),
        name: "elapsedRealtime".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "J".to_owned(),
            return_type: "J".to_owned(),
            parameters: Vec::new(),
        },
    };
    let uptime_millis = DexMethodId {
        class_descriptor: "Landroid/os/SystemClock;".to_owned(),
        name: "uptimeMillis".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "J".to_owned(),
            return_type: "J".to_owned(),
            parameters: Vec::new(),
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&elapsed_realtime, None),
        Some(DexExecutionResult::Value(DexValue::Long(456)))
    );
    assert_eq!(
        program.invoke_builtin_method(&uptime_millis, None),
        Some(DexExecutionResult::Value(DexValue::Long(456)))
    );
}

#[test]
fn builtin_process_get_start_uptime_millis_is_frozen_sample_per_frameworks_base_process_java() {
    let program = DexProgram::new(Vec::new());
    let start_uptime = DexMethodId {
        class_descriptor: "Landroid/os/Process;".to_owned(),
        name: "getStartUptimeMillis".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "J".to_owned(),
            return_type: "J".to_owned(),
            parameters: Vec::new(),
        },
    };
    let uptime_millis = DexMethodId {
        class_descriptor: "Landroid/os/SystemClock;".to_owned(),
        name: "uptimeMillis".to_owned(),
        proto: start_uptime.proto.clone(),
    };

    assert_eq!(
        program.invoke_builtin_method(&start_uptime, None),
        Some(DexExecutionResult::Value(DexValue::Long(0)))
    );

    program.runtime.borrow_mut().main_thread_uptime_millis = 456;

    assert_eq!(
        program.invoke_builtin_method(&uptime_millis, None),
        Some(DexExecutionResult::Value(DexValue::Long(456)))
    );
    assert_eq!(
        program.invoke_builtin_method(&start_uptime, None),
        Some(DexExecutionResult::Value(DexValue::Long(0)))
    );

    program.runtime.borrow_mut().process_start_uptime_millis = 123;

    assert_eq!(
        program.invoke_builtin_method(&start_uptime, None),
        Some(DexExecutionResult::Value(DexValue::Long(123)))
    );
}

#[test]
fn builtin_system_clock_current_thread_time_millis_is_monotonic_per_art_thread_cpu_clock() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/os/SystemClock;".to_owned(),
        name: "currentThreadTimeMillis".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "J".to_owned(),
            return_type: "J".to_owned(),
            parameters: Vec::new(),
        },
    };
    let read = || match program.invoke_builtin_method(&method, None) {
        Some(DexExecutionResult::Value(DexValue::Long(value))) => value,
        other => panic!("unexpected result {other:?}"),
    };

    let first = read();
    let started = std::time::Instant::now();
    while started.elapsed() < std::time::Duration::from_millis(20) {
        std::hint::black_box(0);
    }
    let second = read();

    assert!(first >= 0);
    assert!(second >= first);
}

#[test]
fn builtin_sdk_extensions_get_extension_version_returns_zero() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/os/ext/SdkExtensions;".to_owned(),
        name: "getExtensionVersion".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "II".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let invocation = test_invocation(vec![DexValue::Int(30)]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Int(0))
    );
}

#[test]
fn builtin_looper_get_main_looper_returns_bootstrap_reference() {
    let program = DexProgram::new(Vec::new());
    for name in ["getMainLooper", "myLooper"] {
        let method = DexMethodId {
            class_descriptor: "Landroid/os/Looper;".to_owned(),
            name: name.to_owned(),
            proto: crate::dex::DexProto {
                shorty: "L".to_owned(),
                return_type: "Landroid/os/Looper;".to_owned(),
                parameters: Vec::new(),
            },
        };

        assert_eq!(
            program.invoke_builtin_method(&method, None),
            Some(DexExecutionResult::Value(DexValue::Reference(
                std::rc::Rc::from("Landroid/os/Looper;:main")
            )))
        );
    }
}

#[test]
fn builtin_looper_get_thread_returns_main_thread_reference() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/os/Looper;".to_owned(),
        name: "getThread".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/Thread;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let invocation = test_invocation(vec![DexValue::Reference(std::rc::Rc::from(
        "Landroid/os/Looper;:main",
    ))]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Ljava/lang/Thread;:main")
        )))
    );
}

fn thread_get_thread_group_method() -> DexMethodId {
    DexMethodId {
        class_descriptor: "Ljava/lang/Thread;".to_owned(),
        name: "getThreadGroup".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/ThreadGroup;".to_owned(),
            parameters: Vec::new(),
        },
    }
}

fn main_thread_group() -> Option<DexExecutionResult> {
    Some(DexExecutionResult::Value(DexValue::Reference(
        std::rc::Rc::from("Ljava/lang/ThreadGroup;:main"),
    )))
}

#[test]
fn libcore_thread_get_thread_group_returns_main_group_for_main_and_new_threads() {
    // cite: libcore Thread.java#getThreadGroup
    let program = DexProgram::new(Vec::new());
    let method = thread_get_thread_group_method();
    let main = DexValue::Reference(std::rc::Rc::from("Ljava/lang/Thread;:main"));
    let worker = program.allocate_object("Ljava/lang/Thread;".to_owned());

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![main]))),
        main_thread_group()
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![worker]))),
        main_thread_group()
    );
}

#[test]
fn libcore_thread_get_thread_group_returns_null_after_the_run_completed() {
    // cite: libcore Thread.java#getThreadGroup, getState
    let program = DexProgram::new(Vec::new());
    let method = thread_get_thread_group_method();
    let main = DexValue::Reference(std::rc::Rc::from("Ljava/lang/Thread;:main"));
    let worker = program.allocate_object("Ljava/lang/Thread;".to_owned());

    program
        .finish_started_thread(&worker, DexExecutionResult::Void, "Ljava/lang/Thread;")
        .unwrap();

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![worker]))),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![main]))),
        main_thread_group()
    );
}

#[test]
fn libcore_thread_get_thread_group_rejects_a_non_object_receiver() {
    // cite: libcore Thread.java#getThreadGroup
    let program = DexProgram::new(Vec::new());
    let method = thread_get_thread_group_method();

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![DexValue::Null]))),
        None
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![DexValue::Int(1)]))),
        None
    );
}

#[test]
fn builtin_thread_current_thread_returns_main_thread_reference() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Thread;".to_owned(),
        name: "currentThread".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/Thread;".to_owned(),
            parameters: Vec::new(),
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&method, None),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Ljava/lang/Thread;:main")
        )))
    );
}

#[test]
fn builtin_thread_interrupted_returns_false_without_interrupt_state() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Thread;".to_owned(),
        name: "interrupted".to_owned(),
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

#[test]
fn builtin_thread_is_interrupted_returns_false_without_interrupt_state() {
    // cite: libcore Thread.java#isInterrupted
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Thread;".to_owned(),
        name: "isInterrupted".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let thread = DexValue::Reference(std::rc::Rc::from("Ljava/lang/Thread;:main"));

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![thread]))),
        Some(DexExecutionResult::Int(0))
    );
}

#[test]
fn builtin_thread_uncaught_exception_handler_methods_return_bootstrap_handler() {
    let program = DexProgram::new(Vec::new());
    let get_handler = DexMethodId {
        class_descriptor: "Ljava/lang/Thread;".to_owned(),
        name: "getUncaughtExceptionHandler".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/Thread$UncaughtExceptionHandler;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get_default_handler = DexMethodId {
        class_descriptor: "Ljava/lang/Thread;".to_owned(),
        name: "getDefaultUncaughtExceptionHandler".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/Thread$UncaughtExceptionHandler;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let uncaught_exception = DexMethodId {
        class_descriptor: "Ljava/lang/Thread$UncaughtExceptionHandler;".to_owned(),
        name: "uncaughtException".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VLL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "Ljava/lang/Thread;".to_owned(),
                "Ljava/lang/Throwable;".to_owned(),
            ],
        },
    };
    let handler = DexValue::Reference(std::rc::Rc::from(
        "Ljava/lang/Thread$UncaughtExceptionHandler;:bootstrap",
    ));
    let thread = DexValue::Reference(std::rc::Rc::from("Ljava/lang/Thread;:main"));
    let throwable = DexValue::Object(DexObjectRef {
        id: 7,
        class_descriptor: std::rc::Rc::from("Ljava/lang/Error;"),
    });

    assert_eq!(
        program.invoke_builtin_method(&get_handler, Some(&test_invocation(vec![thread.clone()]))),
        Some(DexExecutionResult::Value(handler.clone()))
    );
    assert_eq!(
        program.invoke_builtin_method(&get_default_handler, None),
        Some(DexExecutionResult::Value(handler.clone()))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &uncaught_exception,
            Some(&test_invocation(vec![handler, thread, throwable]))
        ),
        Some(DexExecutionResult::Void)
    );
}

#[test]
fn builtin_thread_constructors_return_void() {
    let program = DexProgram::new(Vec::new());

    for (descriptor, parameters) in [
        ("()V", Vec::new()),
        (
            "(Ljava/lang/Runnable;)V",
            vec!["Ljava/lang/Runnable;".to_owned()],
        ),
        (
            "(Ljava/lang/String;)V",
            vec!["Ljava/lang/String;".to_owned()],
        ),
        (
            "(Ljava/lang/Runnable;Ljava/lang/String;)V",
            vec![
                "Ljava/lang/Runnable;".to_owned(),
                "Ljava/lang/String;".to_owned(),
            ],
        ),
    ] {
        let method = DexMethodId {
            class_descriptor: "Ljava/lang/Thread;".to_owned(),
            name: "<init>".to_owned(),
            proto: crate::dex::DexProto {
                shorty: "V".to_owned(),
                return_type: "V".to_owned(),
                parameters,
            },
        };

        assert_eq!(method.proto.descriptor(), descriptor);
        assert_eq!(
            program.invoke_builtin_method(&method, None),
            Some(DexExecutionResult::Void)
        );
    }
}

#[test]
fn builtin_thread_constructor_stores_target_and_name() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Ljava/lang/Thread;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VLL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "Ljava/lang/Runnable;".to_owned(),
                "Ljava/lang/String;".to_owned(),
            ],
        },
    };
    let thread = DexValue::Object(DexObjectRef {
        id: 12,
        class_descriptor: std::rc::Rc::from("Ljava/lang/Thread;"),
    });
    let target = DexValue::Object(DexObjectRef {
        id: 13,
        class_descriptor: std::rc::Rc::from("Ljava/lang/Runnable;"),
    });
    let target_field = DexFieldId {
        class_descriptor: "Ljava/lang/Thread;".to_owned(),
        type_descriptor: "Ljava/lang/Runnable;".to_owned(),
        name: "target".to_owned(),
    };
    let name_field = DexFieldId {
        class_descriptor: "Ljava/lang/Thread;".to_owned(),
        type_descriptor: "Ljava/lang/String;".to_owned(),
        name: "name".to_owned(),
    };

    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![
                thread.clone(),
                target.clone(),
                DexValue::String(std::rc::Rc::from("worker"))
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&thread, &target_field),
        Some(target.clone())
    );
    assert_eq!(program.thread_target(&thread), Some(target.clone()));
    assert_eq!(
        program.read_stored_instance_field(&thread, &name_field),
        Some(DexValue::String(std::rc::Rc::from("worker")))
    );
}

#[test]
fn builtin_thread_start_marks_thread_started() {
    let program = DexProgram::new(Vec::new());
    let start = DexMethodId {
        class_descriptor: "Ljava/lang/Thread;".to_owned(),
        name: "start".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let thread = DexValue::Object(DexObjectRef {
        id: 12,
        class_descriptor: std::rc::Rc::from("Lyx;"),
    });
    let started_field = DexFieldId {
        class_descriptor: "Ljava/lang/Thread;".to_owned(),
        type_descriptor: "Z".to_owned(),
        name: "started".to_owned(),
    };

    assert_eq!(
        program.invoke_builtin_method(&start, Some(&test_invocation(vec![thread.clone()]))),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&thread, &started_field),
        Some(DexValue::Int(1))
    );
}

#[test]
fn builtin_thread_set_daemon_stores_flag() {
    let program = DexProgram::new(Vec::new());
    let set_daemon = DexMethodId {
        class_descriptor: "Ljava/lang/Thread;".to_owned(),
        name: "setDaemon".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VZ".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Z".to_owned()],
        },
    };
    let thread = DexValue::Object(DexObjectRef {
        id: 12,
        class_descriptor: std::rc::Rc::from("Ljava/lang/Thread;"),
    });
    let field = DexFieldId {
        class_descriptor: "Ljava/lang/Thread;".to_owned(),
        type_descriptor: "Z".to_owned(),
        name: "daemon".to_owned(),
    };

    assert_eq!(
        program.invoke_builtin_method(
            &set_daemon,
            Some(&test_invocation(vec![thread.clone(), DexValue::Int(7)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&thread, &field),
        Some(DexValue::Int(1))
    );
}

#[test]
fn builtin_thread_set_context_class_loader_stores_loader() {
    let program = DexProgram::new(Vec::new());
    let set_context_class_loader = DexMethodId {
        class_descriptor: "Ljava/lang/Thread;".to_owned(),
        name: "setContextClassLoader".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/lang/ClassLoader;".to_owned()],
        },
    };
    let thread = DexValue::Object(DexObjectRef {
        id: 12,
        class_descriptor: std::rc::Rc::from("Ljava/lang/Thread;"),
    });
    let loader = DexValue::Object(DexObjectRef {
        id: 13,
        class_descriptor: std::rc::Rc::from("Ljava/lang/ClassLoader;"),
    });
    let field = DexFieldId {
        class_descriptor: "Ljava/lang/Thread;".to_owned(),
        type_descriptor: "Ljava/lang/ClassLoader;".to_owned(),
        name: "contextClassLoader".to_owned(),
    };

    assert_eq!(
        program.invoke_builtin_method(
            &set_context_class_loader,
            Some(&test_invocation(vec![thread.clone(), loader.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&thread, &field),
        Some(loader)
    );
}

#[test]
fn builtin_thread_set_name_stores_name() {
    let program = DexProgram::new(Vec::new());
    let set_name = DexMethodId {
        class_descriptor: "Ljava/lang/Thread;".to_owned(),
        name: "setName".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned()],
        },
    };
    let thread = DexValue::Object(DexObjectRef {
        id: 12,
        class_descriptor: std::rc::Rc::from("Ljava/lang/Thread;"),
    });
    let field = DexFieldId {
        class_descriptor: "Ljava/lang/Thread;".to_owned(),
        type_descriptor: "Ljava/lang/String;".to_owned(),
        name: "name".to_owned(),
    };

    assert_eq!(
        program.invoke_builtin_method(
            &set_name,
            Some(&test_invocation(vec![
                thread.clone(),
                DexValue::String(std::rc::Rc::from("DefaultDispatcher-worker-1"))
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&thread, &field),
        Some(DexValue::String(std::rc::Rc::from(
            "DefaultDispatcher-worker-1"
        )))
    );
}

#[test]
fn builtin_thread_get_id_returns_main_thread_id() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Thread;".to_owned(),
        name: "getId".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "J".to_owned(),
            return_type: "J".to_owned(),
            parameters: Vec::new(),
        },
    };
    let invocation = test_invocation(vec![DexValue::Reference(std::rc::Rc::from(
        "Ljava/lang/Thread;:main",
    ))]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Value(DexValue::Long(1)))
    );
}

#[test]
fn builtin_handler_constructors_store_looper_and_post_delayed_queues_runnable() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Landroid/os/Handler;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/os/Looper;".to_owned()],
        },
    };
    let default_constructor = DexMethodId {
        class_descriptor: "Landroid/os/Handler;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let post_delayed = DexMethodId {
        class_descriptor: "Landroid/os/Handler;".to_owned(),
        name: "postDelayed".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZLJ".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/Runnable;".to_owned(), "J".to_owned()],
        },
    };
    let post = DexMethodId {
        class_descriptor: "Landroid/os/Handler;".to_owned(),
        name: "post".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/Runnable;".to_owned()],
        },
    };
    let post_at_front = DexMethodId {
        class_descriptor: "Landroid/os/Handler;".to_owned(),
        name: "postAtFrontOfQueue".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/Runnable;".to_owned()],
        },
    };
    let remove_callbacks = DexMethodId {
        class_descriptor: "Landroid/os/Handler;".to_owned(),
        name: "removeCallbacks".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/lang/Runnable;".to_owned()],
        },
    };
    let get_looper = DexMethodId {
        class_descriptor: "Landroid/os/Handler;".to_owned(),
        name: "getLooper".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/os/Looper;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let obtain_message = DexMethodId {
        class_descriptor: "Landroid/os/Handler;".to_owned(),
        name: "obtainMessage".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LIL".to_owned(),
            return_type: "Landroid/os/Message;".to_owned(),
            parameters: vec!["I".to_owned(), "Ljava/lang/Object;".to_owned()],
        },
    };
    let constructor_invocation = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from("handler")),
        DexValue::Reference(std::rc::Rc::from("Landroid/os/Looper;:main")),
    ]);
    let default_constructor_invocation = test_invocation(vec![DexValue::Reference(
        std::rc::Rc::from("default-handler"),
    )]);
    let get_looper_invocation =
        test_invocation(vec![DexValue::Reference(std::rc::Rc::from("handler"))]);
    let default_get_looper_invocation = test_invocation(vec![DexValue::Reference(
        std::rc::Rc::from("default-handler"),
    )]);
    let post_invocation = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from("handler")),
        DexValue::Reference(std::rc::Rc::from("runnable")),
        DexValue::Long(250),
        DexValue::Long(250),
    ]);
    let second_post_invocation = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from("handler")),
        DexValue::Reference(std::rc::Rc::from("other-runnable")),
        DexValue::Long(500),
        DexValue::Long(500),
    ]);
    let immediate_post_invocation = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from("handler")),
        DexValue::Reference(std::rc::Rc::from("immediate-runnable")),
    ]);
    let front_post_invocation = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from("handler")),
        DexValue::Reference(std::rc::Rc::from("front-runnable")),
    ]);
    let remove_invocation = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from("handler")),
        DexValue::Reference(std::rc::Rc::from("runnable")),
    ]);
    let message_object = DexValue::Reference(std::rc::Rc::from("listener"));

    assert_eq!(
        program.invoke_builtin_method(&constructor, Some(&constructor_invocation)),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&default_constructor, Some(&default_constructor_invocation)),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&get_looper, Some(&get_looper_invocation)),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Landroid/os/Looper;:main")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&get_looper, Some(&default_get_looper_invocation)),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Landroid/os/Looper;:main")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&post_delayed, Some(&post_invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&post_delayed, Some(&second_post_invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&post, Some(&immediate_post_invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&post_at_front, Some(&front_post_invocation)),
        Some(DexExecutionResult::Int(1))
    );
    let Some(DexExecutionResult::Value(message)) = program.invoke_builtin_method(
        &obtain_message,
        Some(&test_invocation(vec![
            DexValue::Reference(std::rc::Rc::from("handler")),
            DexValue::Int(-1),
            message_object.clone(),
        ])),
    ) else {
        panic!("Handler.obtainMessage should return a Message object");
    };
    assert_eq!(
        program.read_stored_instance_field(&message, &message_int_field("what")),
        Some(DexValue::Int(-1))
    );
    assert_eq!(
        program.read_stored_instance_field(&message, &message_object_field("obj")),
        Some(message_object)
    );
    assert_eq!(
        program.read_stored_instance_field(&message, &message_handler_field("target")),
        Some(DexValue::Reference(std::rc::Rc::from("handler")))
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .handler_loopers
            .get("reference:handler"),
        Some(&DexValue::Reference(std::rc::Rc::from(
            "Landroid/os/Looper;:main"
        )))
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .handler_loopers
            .get("reference:default-handler"),
        Some(&DexValue::Reference(std::rc::Rc::from(
            "Landroid/os/Looper;:main"
        )))
    );
    assert_eq!(
        program.runtime.borrow().handler_posts,
        vec![
            HandlerPost {
                handler_key: "reference:handler".to_owned(),
                runnable: DexValue::Reference(std::rc::Rc::from("front-runnable")),
                delay_millis: 0,
                due_time_millis: 0,
                sequence: 4,
                at_front: true,
            },
            HandlerPost {
                handler_key: "reference:handler".to_owned(),
                runnable: DexValue::Reference(std::rc::Rc::from("runnable")),
                delay_millis: 250,
                due_time_millis: 250,
                sequence: 1,
                at_front: false,
            },
            HandlerPost {
                handler_key: "reference:handler".to_owned(),
                runnable: DexValue::Reference(std::rc::Rc::from("other-runnable")),
                delay_millis: 500,
                due_time_millis: 500,
                sequence: 2,
                at_front: false,
            },
            HandlerPost {
                handler_key: "reference:handler".to_owned(),
                runnable: DexValue::Reference(std::rc::Rc::from("immediate-runnable")),
                delay_millis: 0,
                due_time_millis: 0,
                sequence: 3,
                at_front: false,
            }
        ]
    );
    assert_eq!(
        program.invoke_builtin_method(&remove_callbacks, Some(&remove_invocation)),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.runtime.borrow().handler_posts,
        vec![
            HandlerPost {
                handler_key: "reference:handler".to_owned(),
                runnable: DexValue::Reference(std::rc::Rc::from("front-runnable")),
                delay_millis: 0,
                due_time_millis: 0,
                sequence: 4,
                at_front: true,
            },
            HandlerPost {
                handler_key: "reference:handler".to_owned(),
                runnable: DexValue::Reference(std::rc::Rc::from("other-runnable")),
                delay_millis: 500,
                due_time_millis: 500,
                sequence: 2,
                at_front: false,
            },
            HandlerPost {
                handler_key: "reference:handler".to_owned(),
                runnable: DexValue::Reference(std::rc::Rc::from("immediate-runnable")),
                delay_millis: 0,
                due_time_millis: 0,
                sequence: 3,
                at_front: false,
            }
        ]
    );
}

#[test]
fn aosp_messagequeue_androidx_androiduidispatcher_remove_callbacks_uses_runnable_identity() {
    let program = DexProgram::new(Vec::new());
    let remove_callbacks = DexMethodId {
        class_descriptor: "Landroid/os/Handler;".to_owned(),
        name: "removeCallbacks".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/lang/Runnable;".to_owned()],
        },
    };
    let handler = DexValue::Reference(std::rc::Rc::from("Landroid/os/Handler;:compose"));
    let handler_key = map_identity_key(&handler);
    let posted_dispatcher = DexValue::Object(DexObjectRef {
        id: 42,
        class_descriptor: std::rc::Rc::from("Landroidx/compose/ui/platform/AndroidUiDispatcher;"),
    });
    let remove_dispatcher = DexValue::Object(DexObjectRef {
        id: 42,
        class_descriptor: std::rc::Rc::from("Ljava/lang/Runnable;"),
    });
    let other_runnable = DexValue::Object(DexObjectRef {
        id: 43,
        class_descriptor: std::rc::Rc::from("Ljava/lang/Runnable;"),
    });

    {
        let mut runtime = program.runtime.borrow_mut();
        runtime.post_handler_runnable(handler_key.clone(), posted_dispatcher, 0, false);
        runtime.post_handler_runnable(handler_key.clone(), other_runnable.clone(), 0, false);
        runtime
            .choreographer_frame_callbacks
            .push(remove_dispatcher.clone());
    }

    assert_eq!(
        program.invoke_builtin_method(
            &remove_callbacks,
            Some(&test_invocation(vec![handler, remove_dispatcher.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.runtime.borrow().handler_posts,
        vec![HandlerPost {
            handler_key,
            runnable: other_runnable,
            delay_millis: 0,
            due_time_millis: 0,
            sequence: 2,
            at_front: false,
        }]
    );
    assert_eq!(
        program.runtime.borrow().choreographer_frame_callbacks,
        vec![remove_dispatcher]
    );
}

fn instant_to_epoch_milli_method() -> DexMethodId {
    DexMethodId {
        class_descriptor: "Ljava/time/Instant;".to_owned(),
        name: "toEpochMilli".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "J".to_owned(),
            return_type: "J".to_owned(),
            parameters: Vec::new(),
        },
    }
}

fn instant_get_epoch_second_method() -> DexMethodId {
    DexMethodId {
        class_descriptor: "Ljava/time/Instant;".to_owned(),
        name: "getEpochSecond".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "J".to_owned(),
            return_type: "J".to_owned(),
            parameters: Vec::new(),
        },
    }
}

fn instant_get_nano_method() -> DexMethodId {
    DexMethodId {
        class_descriptor: "Ljava/time/Instant;".to_owned(),
        name: "getNano".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    }
}

// cite: libcore Long.java#compare
#[test]
fn long_compare_returns_sign_of_ordering() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Long;".to_owned(),
        name: "compare".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "IJJ".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["J".to_owned(), "J".to_owned()],
        },
    };
    let compare = |left: i64, right: i64| {
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::Long(left),
                DexValue::Long(left),
                DexValue::Long(right),
                DexValue::Long(right),
            ])),
        )
    };

    assert_eq!(compare(1, 2), Some(DexExecutionResult::Int(-1)));
    assert_eq!(compare(2, 2), Some(DexExecutionResult::Int(0)));
    assert_eq!(compare(3, 2), Some(DexExecutionResult::Int(1)));
}

// cite: libcore Integer.java#max, min
#[test]
fn integer_max_and_min_mirror_math() {
    let program = DexProgram::new(Vec::new());
    let build = |name: &str| DexMethodId {
        class_descriptor: "Ljava/lang/Integer;".to_owned(),
        name: name.to_owned(),
        proto: crate::dex::DexProto {
            shorty: "III".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["I".to_owned(), "I".to_owned()],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(
            &build("max"),
            Some(&test_invocation(vec![DexValue::Int(-3), DexValue::Int(7)]))
        ),
        Some(DexExecutionResult::Int(7))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &build("min"),
            Some(&test_invocation(vec![DexValue::Int(-3), DexValue::Int(7)]))
        ),
        Some(DexExecutionResult::Int(-3))
    );
}

// cite: libcore Math.java#abs
#[test]
fn math_abs_long_mirrors_libcore() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Math;".to_owned(),
        name: "abs".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "JJ".to_owned(),
            return_type: "J".to_owned(),
            parameters: vec!["J".to_owned()],
        },
    };
    let invoke = |value: i64| {
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::Long(value),
                DexValue::Long(value),
            ])),
        )
    };

    assert_eq!(
        invoke(-42),
        Some(DexExecutionResult::Value(DexValue::Long(42)))
    );
    assert_eq!(
        invoke(7),
        Some(DexExecutionResult::Value(DexValue::Long(7)))
    );
    assert_eq!(
        invoke(i64::MIN),
        Some(DexExecutionResult::Value(DexValue::Long(i64::MIN)))
    );
}

fn object_wait_contract_program() -> DexProgram {
    let dex_file = crate::dex::parse_dex_file(
        "object_wait_contract.dex",
        include_bytes!("fixtures/object_wait_contract.dex").to_vec(),
    )
    .expect("Object.wait contract DEX should parse");
    DexProgram::new(vec![dex_file])
}

fn execute_object_wait_contract_method(
    program: &DexProgram,
    name: &str,
) -> crate::Result<DexExecutionResult> {
    let method = DexMethodId {
        class_descriptor: "Lwinedroid/tests/ObjectWaitContract;".to_owned(),
        name: name.to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let resolved = program
        .resolve_exact_method(&method)
        .expect("Object.wait contract method should resolve")
        .expect("Object.wait contract method should exist");

    program.execute_method(None, &resolved, None, 0)
}

// cite: libcore Object.java#wait, notifyAll
// cite: libcore Thread.java#start
#[test]
fn libcore_object_wait_returns_after_started_thread_sets_the_awaited_condition() {
    let program = object_wait_contract_program();
    program
        .runtime
        .borrow_mut()
        .post_main_thread_task(MainThreadTask::LaunchActivity {
            class_descriptor: "Lcom/example/Main;".to_owned(),
        });

    assert_eq!(
        execute_object_wait_contract_method(&program, "waitLoopTerminatesAfterStartedThreadRan")
            .expect("wait loop should complete"),
        DexExecutionResult::Int(1)
    );

    let runtime = program.runtime.borrow();
    assert!(runtime
        .main_thread_tasks
        .iter()
        .all(|task| matches!(task, MainThreadTask::LaunchActivity { .. })));
    assert_eq!(runtime.main_thread_tasks.len(), 1);
}

// cite: art monitor.cc#Wait
#[test]
fn art_object_wait_without_notifier_blocks_forever_and_is_an_explicit_error() {
    let program = object_wait_contract_program();

    let error =
        execute_object_wait_contract_method(&program, "waitWithoutRunnableThreadBlocksForever")
            .expect_err("untimed wait with no runnable thread must not return");

    assert!(error
        .to_string()
        .contains("Object.wait would block forever"));
}

// cite: art monitor.cc#Wait
// cite: libcore Object.java#wait
#[test]
fn libcore_object_timed_wait_without_runnable_thread_advances_the_clock_by_the_timeout() {
    let program = object_wait_contract_program();

    assert_eq!(
        execute_object_wait_contract_method(&program, "timedWaitReturnsWithoutRunnableThread")
            .expect("timed wait should return"),
        DexExecutionResult::Int(1)
    );
    assert_eq!(program.runtime.borrow().main_thread_uptime_millis, 50);
}

// cite: libcore Object.java#wait
// cite: art monitor.cc#Wait
#[test]
fn libcore_object_wait_rejects_negative_timeout_and_out_of_range_nanos() {
    let program = object_wait_contract_program();

    for method in [
        "negativeTimeoutThrowsIllegalArgument",
        "outOfRangeNanosThrowsIllegalArgument",
    ] {
        assert_eq!(
            execute_object_wait_contract_method(&program, method).expect("wait should throw"),
            DexExecutionResult::Int(1),
            "{method}"
        );
    }
}

// cite: libcore Object.java#notify
#[test]
fn builtin_object_notify_validates_receiver_like_notify_all() {
    let program = DexProgram::new(Vec::new());
    let notify = DexMethodId {
        class_descriptor: "Ljava/lang/Object;".to_owned(),
        name: "notify".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };

    assert_eq!(
        program.invoke_builtin_method(
            &notify,
            Some(&test_invocation(vec![DexValue::Reference(
                std::rc::Rc::from("lock")
            )]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&notify, Some(&test_invocation(vec![DexValue::Null]))),
        None
    );
}
