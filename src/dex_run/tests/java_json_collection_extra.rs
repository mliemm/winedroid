use super::super::*;
use super::helpers::*;

#[test]
fn builtin_thread_local_stores_value_per_instance() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Ljava/lang/ThreadLocal;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get = DexMethodId {
        class_descriptor: "Ljava/lang/ThreadLocal;".to_owned(),
        name: "get".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set = DexMethodId {
        class_descriptor: "Ljava/lang/ThreadLocal;".to_owned(),
        name: "set".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let remove = DexMethodId {
        class_descriptor: "Ljava/lang/ThreadLocal;".to_owned(),
        name: "remove".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let thread_local = DexValue::Reference(std::rc::Rc::from("Ljava/lang/ThreadLocal;:formatter"));
    let get_invocation = test_invocation(vec![thread_local.clone()]);
    let value = DexValue::Reference(std::rc::Rc::from("Ljava/lang/Object;:value"));
    let set_invocation = test_invocation(vec![thread_local.clone(), value.clone()]);

    assert_eq!(
        program.invoke_builtin_method(&constructor, None),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&get, Some(&get_invocation)),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    assert_eq!(
        program.invoke_builtin_method(&set, Some(&set_invocation)),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&get, Some(&get_invocation)),
        Some(DexExecutionResult::Value(value))
    );
    assert_eq!(
        program.invoke_builtin_method(&remove, Some(&get_invocation)),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&get, Some(&get_invocation)),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
}

#[test]
fn builtin_json_array_serializes_collection_with_boxed_numbers() {
    let program = DexProgram::new(Vec::new());
    let list = DexValue::Reference(std::rc::Rc::from("Ljava/util/ArrayList;:codes"));
    let array = DexValue::Reference(std::rc::Rc::from("Lorg/json/JSONArray;:1"));
    let list_add = DexMethodId {
        class_descriptor: "Ljava/util/List;".to_owned(),
        name: "add".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let init = DexMethodId {
        class_descriptor: "Lorg/json/JSONArray;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/util/Collection;".to_owned()],
        },
    };
    let to_string = DexMethodId {
        class_descriptor: "Lorg/json/JSONArray;".to_owned(),
        name: "toString".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: Vec::new(),
        },
    };

    for value in ["EUR", "GBP"] {
        program.invoke_builtin_method(
            &list_add,
            Some(&test_invocation(vec![
                list.clone(),
                DexValue::String(std::rc::Rc::from(value.to_owned())),
            ])),
        );
    }
    program.invoke_builtin_method(
        &list_add,
        Some(&test_invocation(vec![
            list.clone(),
            DexValue::Reference(std::rc::Rc::from("Ljava/lang/Integer;:42")),
        ])),
    );

    assert_eq!(
        program.invoke_builtin_method(&init, Some(&test_invocation(vec![array.clone(), list]))),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&to_string, Some(&test_invocation(vec![array]))),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("[\"EUR\",\"GBP\",42]")
        )))
    );
}

#[test]
fn builtin_json_object_constructor_returns_void() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Lorg/json/JSONObject;".to_owned(),
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

#[test]
fn libcore_json_object_put_boolean_boxes_and_replaces_value() {
    // cite: libcore JSONObject.java#put, opt, checkName
    let program = DexProgram::new(Vec::new());
    let put = builtin_method(
        "Lorg/json/JSONObject;",
        "put",
        "Lorg/json/JSONObject;",
        &["Ljava/lang/String;", "Z"],
    );
    let opt = builtin_method(
        "Lorg/json/JSONObject;",
        "opt",
        "Ljava/lang/Object;",
        &["Ljava/lang/String;"],
    );
    let to_string = builtin_method(
        "Lorg/json/JSONObject;",
        "toString",
        "Ljava/lang/String;",
        &[],
    );
    let object = program.allocate_object("Lorg/json/JSONObject;".to_owned());
    let key = DexValue::String(std::rc::Rc::from("enabled"));
    for (input, expected) in [(1, "true"), (0, "false")] {
        assert_eq!(
            program.invoke_builtin_method(
                &put,
                Some(&test_invocation(vec![
                    object.clone(),
                    key.clone(),
                    DexValue::Int(input)
                ]))
            ),
            Some(DexExecutionResult::Value(object.clone()))
        );
        assert_eq!(
            program.invoke_builtin_method(
                &opt,
                Some(&test_invocation(vec![object.clone(), key.clone()]))
            ),
            Some(DexExecutionResult::Value(DexValue::Reference(
                std::rc::Rc::from(format!("Ljava/lang/Boolean;:{expected}"))
            )))
        );
        assert_eq!(
            program.invoke_builtin_method(&to_string, Some(&test_invocation(vec![object.clone()]))),
            Some(DexExecutionResult::Value(DexValue::String(
                std::rc::Rc::from(format!("{{\"enabled\":{expected}}}"))
            )))
        );
    }
    assert!(matches!(
        program.invoke_builtin_method(
            &put,
            Some(&test_invocation(vec![
                object,
                DexValue::Null,
                DexValue::Int(1)
            ]))
        ),
        Some(DexExecutionResult::Throw(_))
    ));
}

#[test]
fn builtin_json_object_put_int_stores_value_and_returns_object() {
    let program = DexProgram::new(Vec::new());
    let put_int = DexMethodId {
        class_descriptor: "Lorg/json/JSONObject;".to_owned(),
        name: "put".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LLI".to_owned(),
            return_type: "Lorg/json/JSONObject;".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned(), "I".to_owned()],
        },
    };
    let put_object = DexMethodId {
        class_descriptor: "Lorg/json/JSONObject;".to_owned(),
        name: "put".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LLL".to_owned(),
            return_type: "Lorg/json/JSONObject;".to_owned(),
            parameters: vec![
                "Ljava/lang/String;".to_owned(),
                "Ljava/lang/Object;".to_owned(),
            ],
        },
    };
    let put_long = DexMethodId {
        class_descriptor: "Lorg/json/JSONObject;".to_owned(),
        name: "put".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LLJ".to_owned(),
            return_type: "Lorg/json/JSONObject;".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned(), "J".to_owned()],
        },
    };
    let length = DexMethodId {
        class_descriptor: "Lorg/json/JSONObject;".to_owned(),
        name: "length".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let is_null = DexMethodId {
        class_descriptor: "Lorg/json/JSONObject;".to_owned(),
        name: "isNull".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned()],
        },
    };
    let to_string = DexMethodId {
        class_descriptor: "Lorg/json/JSONObject;".to_owned(),
        name: "toString".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let object = DexValue::Reference(std::rc::Rc::from("json"));
    let put_int_invocation = test_invocation(vec![
        object.clone(),
        DexValue::String(std::rc::Rc::from("elapsed")),
        DexValue::Int(7),
    ]);
    let put_object_invocation = test_invocation(vec![
        object.clone(),
        DexValue::String(std::rc::Rc::from("country")),
        DexValue::String(std::rc::Rc::from("US")),
    ]);
    let put_long_invocation = test_invocation(vec![
        object.clone(),
        DexValue::String(std::rc::Rc::from("duration")),
        DexValue::Long(42),
        DexValue::Long(42),
    ]);
    let length_invocation = test_invocation(vec![DexValue::Reference(std::rc::Rc::from("json"))]);
    let present_invocation = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from("json")),
        DexValue::String(std::rc::Rc::from("country")),
    ]);
    let missing_invocation = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from("json")),
        DexValue::String(std::rc::Rc::from("missing")),
    ]);

    assert_eq!(
        program.invoke_builtin_method(&put_int, Some(&put_int_invocation)),
        Some(DexExecutionResult::Value(object.clone()))
    );
    assert_eq!(
        program.invoke_builtin_method(&put_object, Some(&put_object_invocation)),
        Some(DexExecutionResult::Value(object))
    );
    assert_eq!(
        program.invoke_builtin_method(&put_long, Some(&put_long_invocation)),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("json")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&length, Some(&length_invocation)),
        Some(DexExecutionResult::Int(3))
    );
    assert_eq!(
        program.invoke_builtin_method(&is_null, Some(&present_invocation)),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&is_null, Some(&missing_invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&to_string, Some(&length_invocation)),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("{\"country\":\"US\",\"duration\":42,\"elapsed\":7}")
        )))
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .json_entries
            .get(&("reference:json".to_owned(), "elapsed".to_owned())),
        Some(&DexValue::Int(7))
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .json_entries
            .get(&("reference:json".to_owned(), "country".to_owned())),
        Some(&DexValue::String(std::rc::Rc::from("US")))
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .json_entries
            .get(&("reference:json".to_owned(), "duration".to_owned())),
        Some(&DexValue::Long(42))
    );
}

#[test]
fn builtin_json_object_to_string_serializes_nested_json_objects() {
    let program = DexProgram::new(Vec::new());
    let put_int = DexMethodId {
        class_descriptor: "Lorg/json/JSONObject;".to_owned(),
        name: "put".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LLI".to_owned(),
            return_type: "Lorg/json/JSONObject;".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned(), "I".to_owned()],
        },
    };
    let put_object = DexMethodId {
        class_descriptor: "Lorg/json/JSONObject;".to_owned(),
        name: "put".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LLL".to_owned(),
            return_type: "Lorg/json/JSONObject;".to_owned(),
            parameters: vec![
                "Ljava/lang/String;".to_owned(),
                "Ljava/lang/Object;".to_owned(),
            ],
        },
    };
    let to_string = DexMethodId {
        class_descriptor: "Lorg/json/JSONObject;".to_owned(),
        name: "toString".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let parent = DexValue::Object(DexObjectRef {
        id: 1,
        class_descriptor: std::rc::Rc::from("Lorg/json/JSONObject;"),
    });
    let child = DexValue::Object(DexObjectRef {
        id: 2,
        class_descriptor: std::rc::Rc::from("Lorg/json/JSONObject;"),
    });
    let child_put_invocation = test_invocation(vec![
        child.clone(),
        DexValue::String(std::rc::Rc::from("count")),
        DexValue::Int(3),
    ]);
    let parent_put_invocation = test_invocation(vec![
        parent.clone(),
        DexValue::String(std::rc::Rc::from("child")),
        child,
    ]);
    let to_string_invocation = test_invocation(vec![parent]);

    assert_eq!(
        program.invoke_builtin_method(&put_int, Some(&child_put_invocation)),
        Some(DexExecutionResult::Value(DexValue::Object(DexObjectRef {
            id: 2,
            class_descriptor: std::rc::Rc::from("Lorg/json/JSONObject;")
        })))
    );
    assert_eq!(
        program.invoke_builtin_method(&put_object, Some(&parent_put_invocation)),
        Some(DexExecutionResult::Value(DexValue::Object(DexObjectRef {
            id: 1,
            class_descriptor: std::rc::Rc::from("Lorg/json/JSONObject;")
        })))
    );
    assert_eq!(
        program.invoke_builtin_method(&to_string, Some(&to_string_invocation)),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("{\"child\":{\"count\":3}}")
        )))
    );
}

#[test]
fn builtin_json_array_parses_string_and_reads_back_elements() {
    let program = DexProgram::new(Vec::new());
    let array = DexValue::Reference(std::rc::Rc::from("Lorg/json/JSONArray;:names"));
    let init_string = DexMethodId {
        class_descriptor: "Lorg/json/JSONArray;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned()],
        },
    };
    let length = DexMethodId {
        class_descriptor: "Lorg/json/JSONArray;".to_owned(),
        name: "length".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get_string = DexMethodId {
        class_descriptor: "Lorg/json/JSONArray;".to_owned(),
        name: "getString".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LI".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let is_null = DexMethodId {
        class_descriptor: "Lorg/json/JSONArray;".to_owned(),
        name: "isNull".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZI".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };

    program.invoke_builtin_method(
        &init_string,
        Some(&test_invocation(vec![
            array.clone(),
            DexValue::String(std::rc::Rc::from(r#"["GBP","CAD","USD","EUR"]"#.to_owned())),
        ])),
    );

    assert_eq!(
        program.invoke_builtin_method(&length, Some(&test_invocation(vec![array.clone()]))),
        Some(DexExecutionResult::Int(4))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_string,
            Some(&test_invocation(vec![array.clone(), DexValue::Int(1)]))
        ),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("CAD")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &is_null,
            Some(&test_invocation(vec![array.clone(), DexValue::Int(2)]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &is_null,
            Some(&test_invocation(vec![array, DexValue::Int(9)]))
        ),
        Some(DexExecutionResult::Int(1))
    );
}

fn thread_local_contract_result(method: &str) -> DexExecutionResult {
    let dex_file = crate::dex::parse_dex_file(
        "thread_local_contract.dex",
        include_bytes!("fixtures/thread_local_contract.dex").to_vec(),
    )
    .expect("ThreadLocal contract DEX should parse");
    let program = DexProgram::new(vec![dex_file]);
    let method = DexMethodId {
        class_descriptor: "Lwinedroid/tests/ThreadLocalContract;".to_owned(),
        name: method.to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let resolved = program
        .resolve_exact_method(&method)
        .expect("ThreadLocal contract method should resolve")
        .expect("ThreadLocal contract method should exist");

    program
        .execute_method(None, &resolved, None, 0)
        .expect("ThreadLocal contract method should execute")
}

// cite: libcore ThreadLocal.java#get, setInitialValue, initialValue, remove
#[test]
fn libcore_thread_local_get_runs_guest_initial_value_once_until_remove() {
    assert_eq!(
        thread_local_contract_result("initialValueRunsOncePerGetUntilRemove"),
        DexExecutionResult::Int(2)
    );
}

// cite: libcore ThreadLocal.java#get, setInitialValue
#[test]
fn libcore_thread_local_get_propagates_initial_value_exception_and_stores_nothing() {
    assert_eq!(
        thread_local_contract_result("throwingInitialValuePropagatesAndStoresNothing"),
        DexExecutionResult::Int(2)
    );
}

// cite: libcore ThreadLocal.java#initialValue, set
#[test]
fn libcore_thread_local_default_initial_value_is_null() {
    assert_eq!(
        thread_local_contract_result("defaultInitialValueIsNull"),
        DexExecutionResult::Int(1)
    );
}

fn json_object_from_map(
    program: &DexProgram,
    entries: &[(&str, DexValue)],
) -> (DexValue, Option<DexExecutionResult>) {
    let entries: Vec<(DexValue, DexValue)> = entries
        .iter()
        .map(|(key, value)| (DexValue::String(std::rc::Rc::from(*key)), value.clone()))
        .collect();

    json_object_from_keyed_map(program, &entries)
}

fn json_object_from_keyed_map(
    program: &DexProgram,
    entries: &[(DexValue, DexValue)],
) -> (DexValue, Option<DexExecutionResult>) {
    let map = program.allocate_object("Ljava/util/HashMap;".to_owned());
    let map_constructor = builtin_method("Ljava/util/HashMap;", "<init>", "V", &[]);
    program.invoke_builtin_method(&map_constructor, Some(&test_invocation(vec![map.clone()])));
    let put = builtin_method(
        "Ljava/util/HashMap;",
        "put",
        "Ljava/lang/Object;",
        &["Ljava/lang/Object;", "Ljava/lang/Object;"],
    );
    for (key, value) in entries {
        program.invoke_builtin_method(
            &put,
            Some(&test_invocation(vec![
                map.clone(),
                key.clone(),
                value.clone(),
            ])),
        );
    }

    let object = program.allocate_object("Lorg/json/JSONObject;".to_owned());
    let constructor = builtin_method("Lorg/json/JSONObject;", "<init>", "V", &["Ljava/util/Map;"]);
    let result = program.invoke_builtin_method(
        &constructor,
        Some(&test_invocation(vec![object.clone(), map])),
    );

    (object, result)
}

fn json_to_string(program: &DexProgram, object: &DexValue) -> Option<DexExecutionResult> {
    let to_string = builtin_method(
        "Lorg/json/JSONObject;",
        "toString",
        "Ljava/lang/String;",
        &[],
    );

    program.invoke_builtin_method(&to_string, Some(&test_invocation(vec![object.clone()])))
}

#[test]
fn libcore_json_object_map_constructor_copies_wrapped_entries() {
    // cite: libcore JSONObject.java#JSONObject, wrap
    let program = DexProgram::new(Vec::new());
    let (object, result) = json_object_from_map(
        &program,
        &[
            ("a", DexValue::String(std::rc::Rc::from("x"))),
            (
                "b",
                DexValue::Reference(std::rc::Rc::from("Ljava/lang/Boolean;:true")),
            ),
        ],
    );

    assert_eq!(result, Some(DexExecutionResult::Void));
    assert_eq!(
        json_to_string(&program, &object),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("{\"a\":\"x\",\"b\":true}")
        )))
    );
}

#[test]
fn libcore_json_object_map_constructor_null_map_throws_null_pointer_exception() {
    // cite: libcore JSONObject.java#JSONObject
    let program = DexProgram::new(Vec::new());
    let object = program.allocate_object("Lorg/json/JSONObject;".to_owned());
    let constructor = builtin_method("Lorg/json/JSONObject;", "<init>", "V", &["Ljava/util/Map;"]);

    let Some(DexExecutionResult::Throw(exception)) = program.invoke_builtin_method(
        &constructor,
        Some(&test_invocation(vec![object, DexValue::Null])),
    ) else {
        panic!("null map should throw");
    };
    assert!(program.value_matches_type(&exception, "Ljava/lang/NullPointerException;"));
}

#[test]
fn libcore_json_object_map_constructor_unwrappable_value_is_a_missing_primitive() {
    // cite: libcore JSONObject.java#wrap
    let program = DexProgram::new(Vec::new());
    let (_, result) = json_object_from_map(&program, &[("a", DexValue::Float(1.5))]);

    assert_eq!(result, None);
}

#[test]
fn libcore_json_object_map_constructor_null_key_throws_null_pointer_exception() {
    // cite: libcore JSONObject.java#JSONObject
    let program = DexProgram::new(Vec::new());
    let (_, result) = json_object_from_keyed_map(
        &program,
        &[(DexValue::Null, DexValue::String(std::rc::Rc::from("x")))],
    );

    let Some(DexExecutionResult::Throw(exception)) = result else {
        panic!("null key should throw: {result:?}");
    };
    assert!(program.value_matches_type(&exception, "Ljava/lang/NullPointerException;"));
}

#[test]
fn org_json_json_object_has_reports_stored_keys_including_null_values() {
    // cite: libcore JSONObject.java#has
    let program = DexProgram::new(Vec::new());
    let put = builtin_method(
        "Lorg/json/JSONObject;",
        "put",
        "Lorg/json/JSONObject;",
        &["Ljava/lang/String;", "Ljava/lang/Object;"],
    );
    let has = builtin_method("Lorg/json/JSONObject;", "has", "Z", &["Ljava/lang/String;"]);
    let json = program.allocate_object("Lorg/json/JSONObject;".to_owned());
    let key = DexValue::String(std::rc::Rc::from("present"));

    assert_eq!(
        program.invoke_builtin_method(
            &put,
            Some(&test_invocation(vec![
                json.clone(),
                key.clone(),
                DexValue::Null
            ]))
        ),
        Some(DexExecutionResult::Value(json.clone()))
    );
    assert_eq!(
        program.invoke_builtin_method(&has, Some(&test_invocation(vec![json.clone(), key]))),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &has,
            Some(&test_invocation(vec![
                json,
                DexValue::String(std::rc::Rc::from("missing"))
            ]))
        ),
        Some(DexExecutionResult::Int(0))
    );
}

#[test]
fn org_json_json_object_opt_returns_the_stored_value_or_null() {
    // cite: libcore JSONObject.java#opt
    let program = DexProgram::new(Vec::new());
    let put = builtin_method(
        "Lorg/json/JSONObject;",
        "put",
        "Lorg/json/JSONObject;",
        &["Ljava/lang/String;", "Ljava/lang/Object;"],
    );
    let opt = builtin_method(
        "Lorg/json/JSONObject;",
        "opt",
        "Ljava/lang/Object;",
        &["Ljava/lang/String;"],
    );
    let json = program.allocate_object("Lorg/json/JSONObject;".to_owned());
    let key = DexValue::String(std::rc::Rc::from("present"));
    let value = DexValue::String(std::rc::Rc::from("value"));

    assert_eq!(
        program.invoke_builtin_method(
            &put,
            Some(&test_invocation(vec![
                json.clone(),
                key.clone(),
                value.clone()
            ]))
        ),
        Some(DexExecutionResult::Value(json.clone()))
    );
    assert_eq!(
        program.invoke_builtin_method(&opt, Some(&test_invocation(vec![json.clone(), key]))),
        Some(DexExecutionResult::Value(value))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &opt,
            Some(&test_invocation(vec![
                json,
                DexValue::String(std::rc::Rc::from("missing"))
            ]))
        ),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
}

#[test]
fn org_json_json_object_opt_json_object_returns_only_nested_objects() {
    // cite: libcore JSONObject.java#optJSONObject
    let program = DexProgram::new(Vec::new());
    let put = builtin_method(
        "Lorg/json/JSONObject;",
        "put",
        "Lorg/json/JSONObject;",
        &["Ljava/lang/String;", "Ljava/lang/Object;"],
    );
    let opt_json_object = builtin_method(
        "Lorg/json/JSONObject;",
        "optJSONObject",
        "Lorg/json/JSONObject;",
        &["Ljava/lang/String;"],
    );
    let json = program.allocate_object("Lorg/json/JSONObject;".to_owned());
    let nested = program.allocate_object("Lorg/json/JSONObject;".to_owned());
    let key = DexValue::String(std::rc::Rc::from("nested"));

    assert_eq!(
        program.invoke_builtin_method(
            &put,
            Some(&test_invocation(vec![
                json.clone(),
                key.clone(),
                nested.clone()
            ]))
        ),
        Some(DexExecutionResult::Value(json.clone()))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &opt_json_object,
            Some(&test_invocation(vec![json.clone(), key]))
        ),
        Some(DexExecutionResult::Value(nested))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &opt_json_object,
            Some(&test_invocation(vec![
                json,
                DexValue::String(std::rc::Rc::from("missing"))
            ]))
        ),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
}
