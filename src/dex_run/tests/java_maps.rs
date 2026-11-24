use super::super::*;
use super::helpers::*;

#[test]
fn java_collection_interface_pre_dispatch_includes_map() {
    let method = DexMethodId {
        class_descriptor: "Ljava/util/Map;".to_owned(),
        name: "get".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };

    assert!(java_collection_interface_method(&method));
}

#[test]
fn builtin_collections_synchronized_map_returns_same_map_reference() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/util/Collections;".to_owned(),
        name: "synchronizedMap".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/util/Map;".to_owned(),
            parameters: vec!["Ljava/util/Map;".to_owned()],
        },
    };
    let map = DexValue::Object(DexObjectRef {
        id: 1,
        class_descriptor: std::rc::Rc::from("Ljava/util/HashMap;"),
    });
    let invocation = test_invocation(vec![map.clone()]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Value(map))
    );
}

#[test]
fn builtin_collections_synchronized_set_and_list_return_backing_view_per_libcore_collections() {
    let program = DexProgram::new(Vec::new());
    for (name, interface, backing) in [
        (
            "synchronizedSet",
            "Ljava/util/Set;",
            "Ljava/util/HashSet;",
        ),
        (
            "synchronizedList",
            "Ljava/util/List;",
            "Ljava/util/ArrayList;",
        ),
    ] {
        let method = DexMethodId {
            class_descriptor: "Ljava/util/Collections;".to_owned(),
            name: name.to_owned(),
            proto: crate::dex::DexProto {
                shorty: "LL".to_owned(),
                return_type: interface.to_owned(),
                parameters: vec![interface.to_owned()],
            },
        };
        let collection = DexValue::Object(DexObjectRef {
            id: 1,
            class_descriptor: std::rc::Rc::from(backing),
        });
        let invocation = test_invocation(vec![collection.clone()]);

        assert_eq!(
            program.invoke_builtin_method(&method, Some(&invocation)),
            Some(DexExecutionResult::Value(collection))
        );
        assert_eq!(program.invoke_builtin_method(&method, None), None);
    }
}

#[test]
fn builtin_collections_unmodifiable_map_returns_same_map_reference() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/util/Collections;".to_owned(),
        name: "unmodifiableMap".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/util/Map;".to_owned(),
            parameters: vec!["Ljava/util/Map;".to_owned()],
        },
    };
    let map = DexValue::Object(DexObjectRef {
        id: 1,
        class_descriptor: std::rc::Rc::from("Ljava/util/HashMap;"),
    });
    let invocation = test_invocation(vec![map.clone()]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Value(map))
    );
}

#[test]
fn builtin_abstract_map_constructor_returns_void() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/util/AbstractMap;".to_owned(),
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
fn runtime_object_equals_compares_managed_map_entries() {
    let program = DexProgram::new(Vec::new());
    let put = DexMethodId {
        class_descriptor: "Ljava/util/Map;".to_owned(),
        name: "put".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LLL".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: vec![
                "Ljava/lang/Object;".to_owned(),
                "Ljava/lang/Object;".to_owned(),
            ],
        },
    };
    let left = DexValue::Object(DexObjectRef {
        id: 1,
        class_descriptor: std::rc::Rc::from("Ljava/util/LinkedHashMap;"),
    });
    let right = DexValue::Object(DexObjectRef {
        id: 2,
        class_descriptor: std::rc::Rc::from("Ljava/util/LinkedHashMap;"),
    });

    for map in [left.clone(), right.clone()] {
        assert_eq!(
            program.invoke_builtin_method(
                &put,
                Some(&test_invocation(vec![
                    map,
                    DexValue::String(std::rc::Rc::from("name")),
                    DexValue::String(std::rc::Rc::from("TEXT")),
                ])),
            ),
            Some(DexExecutionResult::Value(DexValue::Null))
        );
    }

    assert_eq!(
        program
            .runtime_object_equals(&test_invocation(vec![left.clone(), right.clone()]), 0)
            .expect("map equality should run"),
        1
    );
    assert_eq!(
        program.invoke_builtin_method(
            &put,
            Some(&test_invocation(vec![
                right.clone(),
                DexValue::String(std::rc::Rc::from("name")),
                DexValue::String(std::rc::Rc::from("INTEGER")),
            ])),
        ),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("TEXT")
        )))
    );
    assert_eq!(
        program
            .runtime_object_equals(&test_invocation(vec![left, right]), 0)
            .expect("map equality should run"),
        0
    );
}

#[test]
fn builtin_map_matches_class_backed_wrapper_keys_by_class_reference() {
    let program = DexProgram::new(Vec::new());
    let get = DexMethodId {
        class_descriptor: "Ljava/util/Map;".to_owned(),
        name: "get".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let put = DexMethodId {
        class_descriptor: "Ljava/util/Map;".to_owned(),
        name: "put".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LLL".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: vec![
                "Ljava/lang/Object;".to_owned(),
                "Ljava/lang/Object;".to_owned(),
            ],
        },
    };
    let contains_key = DexMethodId {
        class_descriptor: "Ljava/util/Map;".to_owned(),
        name: "containsKey".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let map = DexValue::Reference(std::rc::Rc::from("map"));
    let stored_key = program.allocate_object("Lgp;".to_owned());
    let lookup_key = program.allocate_object("Lgp;".to_owned());
    let other_key = program.allocate_object("Lgp;".to_owned());
    let class_field = DexFieldId {
        class_descriptor: "Lgp;".to_owned(),
        type_descriptor: "Ljava/lang/Class;".to_owned(),
        name: "a".to_owned(),
    };
    let class_reference = DexValue::Reference(std::rc::Rc::from("Ljava/lang/Class;:Lnq0;"));
    let other_class_reference = DexValue::Reference(std::rc::Rc::from("Ljava/lang/Class;:Lother;"));
    let value = DexValue::Reference(std::rc::Rc::from("value"));

    assert!(program.write_stored_instance_field(
        &stored_key,
        &class_field,
        class_reference.clone()
    ));
    assert!(program.write_stored_instance_field(&lookup_key, &class_field, class_reference));
    assert!(program.write_stored_instance_field(&other_key, &class_field, other_class_reference));

    assert_eq!(
        program.invoke_builtin_method(
            &put,
            Some(&test_invocation(vec![
                map.clone(),
                stored_key,
                value.clone()
            ])),
        ),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get,
            Some(&test_invocation(vec![map.clone(), lookup_key.clone()])),
        ),
        Some(DexExecutionResult::Value(value))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &contains_key,
            Some(&test_invocation(vec![map.clone(), lookup_key])),
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&get, Some(&test_invocation(vec![map, other_key]))),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
}

#[test]
fn builtin_map_matches_class_and_value_wrapper_keys() {
    let program = DexProgram::new(Vec::new());
    let get = DexMethodId {
        class_descriptor: "Ljava/util/Map;".to_owned(),
        name: "get".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let put = DexMethodId {
        class_descriptor: "Ljava/util/Map;".to_owned(),
        name: "put".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LLL".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: vec![
                "Ljava/lang/Object;".to_owned(),
                "Ljava/lang/Object;".to_owned(),
            ],
        },
    };
    let contains_key = DexMethodId {
        class_descriptor: "Ljava/util/Map;".to_owned(),
        name: "containsKey".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let class_field = DexFieldId {
        class_descriptor: "Lgp;".to_owned(),
        type_descriptor: "Ljava/lang/Class;".to_owned(),
        name: "a".to_owned(),
    };
    let wrapper_class_field = DexFieldId {
        class_descriptor: "Lyb;".to_owned(),
        type_descriptor: "Lgp;".to_owned(),
        name: "a".to_owned(),
    };
    let wrapper_value_field = DexFieldId {
        class_descriptor: "Lyb;".to_owned(),
        type_descriptor: "Ljava/lang/Object;".to_owned(),
        name: "b".to_owned(),
    };
    let map = DexValue::Reference(std::rc::Rc::from("map"));
    let stored_class = program.allocate_object("Lgp;".to_owned());
    let lookup_class = program.allocate_object("Lgp;".to_owned());
    let other_class = program.allocate_object("Lgp;".to_owned());
    let stored_key = program.allocate_object("Lyb;".to_owned());
    let lookup_key = program.allocate_object("Lyb;".to_owned());
    let other_key = program.allocate_object("Lyb;".to_owned());
    let class_reference = DexValue::Reference(std::rc::Rc::from("Ljava/lang/Class;:Lgi2;"));
    let other_class_reference = DexValue::Reference(std::rc::Rc::from("Ljava/lang/Class;:Lother;"));
    let route_key = DexValue::String(std::rc::Rc::from("nq0@route"));
    let value = DexValue::Reference(std::rc::Rc::from("value"));

    assert!(program.write_stored_instance_field(
        &stored_class,
        &class_field,
        class_reference.clone()
    ));
    assert!(program.write_stored_instance_field(&lookup_class, &class_field, class_reference));
    assert!(program.write_stored_instance_field(&other_class, &class_field, other_class_reference));
    assert!(program.write_stored_instance_field(&stored_key, &wrapper_class_field, stored_class));
    assert!(program.write_stored_instance_field(
        &stored_key,
        &wrapper_value_field,
        route_key.clone()
    ));
    assert!(program.write_stored_instance_field(&lookup_key, &wrapper_class_field, lookup_class));
    assert!(program.write_stored_instance_field(
        &lookup_key,
        &wrapper_value_field,
        route_key.clone()
    ));
    assert!(program.write_stored_instance_field(&other_key, &wrapper_class_field, other_class));
    assert!(program.write_stored_instance_field(&other_key, &wrapper_value_field, route_key));

    assert_eq!(
        program.invoke_builtin_method(
            &put,
            Some(&test_invocation(vec![
                map.clone(),
                stored_key,
                value.clone()
            ])),
        ),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get,
            Some(&test_invocation(vec![map.clone(), lookup_key.clone()])),
        ),
        Some(DexExecutionResult::Value(value))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &contains_key,
            Some(&test_invocation(vec![map.clone(), lookup_key])),
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&contains_key, Some(&test_invocation(vec![map, other_key]))),
        Some(DexExecutionResult::Int(0))
    );
}

#[test]
fn builtin_hash_map_put_stores_value_for_later_get() {
    let program = DexProgram::new(Vec::new());
    let get = DexMethodId {
        class_descriptor: "Ljava/util/HashMap;".to_owned(),
        name: "get".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let put = DexMethodId {
        class_descriptor: "Ljava/util/HashMap;".to_owned(),
        name: "put".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LLL".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: vec![
                "Ljava/lang/Object;".to_owned(),
                "Ljava/lang/Object;".to_owned(),
            ],
        },
    };
    let weak_hash_map_get = DexMethodId {
        class_descriptor: "Ljava/util/WeakHashMap;".to_owned(),
        name: "get".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let weak_hash_map_put = DexMethodId {
        class_descriptor: "Ljava/util/WeakHashMap;".to_owned(),
        name: "put".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LLL".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: vec![
                "Ljava/lang/Object;".to_owned(),
                "Ljava/lang/Object;".to_owned(),
            ],
        },
    };
    let weak_hash_map_is_empty = DexMethodId {
        class_descriptor: "Ljava/util/WeakHashMap;".to_owned(),
        name: "isEmpty".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let weak_hash_map_key_set = DexMethodId {
        class_descriptor: "Ljava/util/WeakHashMap;".to_owned(),
        name: "keySet".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/util/Set;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set_iterator = DexMethodId {
        class_descriptor: "Ljava/util/Set;".to_owned(),
        name: "iterator".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/util/Iterator;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let iterator_next = DexMethodId {
        class_descriptor: "Ljava/util/Iterator;".to_owned(),
        name: "next".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let map_put = DexMethodId {
        class_descriptor: "Ljava/util/Map;".to_owned(),
        name: "put".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LLL".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: vec![
                "Ljava/lang/Object;".to_owned(),
                "Ljava/lang/Object;".to_owned(),
            ],
        },
    };
    let abstract_map_put = DexMethodId {
        class_descriptor: "Ljava/util/AbstractMap;".to_owned(),
        name: "put".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LLL".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: vec![
                "Ljava/lang/Object;".to_owned(),
                "Ljava/lang/Object;".to_owned(),
            ],
        },
    };
    let contains_key = DexMethodId {
        class_descriptor: "Ljava/util/Map;".to_owned(),
        name: "containsKey".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let map_size = DexMethodId {
        class_descriptor: "Ljava/util/Map;".to_owned(),
        name: "size".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let hash_map_size = DexMethodId {
        class_descriptor: "Ljava/util/HashMap;".to_owned(),
        name: "size".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let hash_map_remove = DexMethodId {
        class_descriptor: "Ljava/util/HashMap;".to_owned(),
        name: "remove".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let hash_map_clear = DexMethodId {
        class_descriptor: "Ljava/util/HashMap;".to_owned(),
        name: "clear".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let hash_map_copy_constructor = DexMethodId {
        class_descriptor: "Ljava/util/HashMap;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/util/Map;".to_owned()],
        },
    };
    let linked_hash_map_copy_constructor = DexMethodId {
        class_descriptor: "Ljava/util/LinkedHashMap;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/util/Map;".to_owned()],
        },
    };
    let hash_map_put_all = DexMethodId {
        class_descriptor: "Ljava/util/HashMap;".to_owned(),
        name: "putAll".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/util/Map;".to_owned()],
        },
    };
    let map = DexValue::Reference(std::rc::Rc::from("map"));
    let key = DexValue::Reference(std::rc::Rc::from("key"));
    let value = DexValue::Reference(std::rc::Rc::from("value"));
    let get_invocation = test_invocation(vec![map.clone(), key.clone()]);
    let put_invocation = test_invocation(vec![map.clone(), key.clone(), value.clone()]);
    let size_invocation = test_invocation(vec![map.clone()]);
    let replace_invocation = test_invocation(vec![
        map,
        key,
        DexValue::Reference(std::rc::Rc::from("replacement")),
    ]);
    let linked_map = DexValue::Reference(std::rc::Rc::from("linked-map"));
    let linked_key = DexValue::String(std::rc::Rc::from("enabled"));
    let linked_value = DexValue::Reference(std::rc::Rc::from("Ljava/lang/Boolean;:true"));
    let linked_put_invocation = test_invocation(vec![
        linked_map.clone(),
        linked_key.clone(),
        linked_value.clone(),
    ]);
    let linked_size_invocation = test_invocation(vec![linked_map.clone()]);
    let linked_get_invocation = test_invocation(vec![linked_map.clone(), linked_key.clone()]);
    let copied_map = DexValue::Reference(std::rc::Rc::from("copied-map"));
    let copy_constructor_invocation = test_invocation(vec![copied_map.clone(), linked_map]);
    let copied_size_invocation = test_invocation(vec![copied_map.clone()]);
    let copied_get_invocation = test_invocation(vec![copied_map, linked_key.clone()]);
    let copied_linked_map = DexValue::Reference(std::rc::Rc::from("copied-linked-map"));
    let linked_copy_constructor_invocation = test_invocation(vec![
        copied_linked_map.clone(),
        DexValue::Reference(std::rc::Rc::from("map")),
    ]);
    let copied_linked_get_invocation = test_invocation(vec![
        copied_linked_map.clone(),
        DexValue::Reference(std::rc::Rc::from("key")),
    ]);
    let copied_linked_size_invocation = test_invocation(vec![copied_linked_map]);
    let put_all_map = DexValue::Reference(std::rc::Rc::from("put-all-map"));
    let put_all_invocation = test_invocation(vec![
        put_all_map.clone(),
        DexValue::Reference(std::rc::Rc::from("map")),
    ]);
    let put_all_get_invocation = test_invocation(vec![
        put_all_map.clone(),
        DexValue::Reference(std::rc::Rc::from("key")),
    ]);
    let put_all_size_invocation = test_invocation(vec![put_all_map.clone()]);
    let put_all_remove_invocation = test_invocation(vec![
        put_all_map,
        DexValue::Reference(std::rc::Rc::from("key")),
    ]);
    let clear_invocation = test_invocation(vec![DexValue::Reference(std::rc::Rc::from("map"))]);
    let abstract_map = DexValue::Reference(std::rc::Rc::from("abstract-map-dispatch"));
    let abstract_key = DexValue::Reference(std::rc::Rc::from("abstract-key"));
    let abstract_value = DexValue::Reference(std::rc::Rc::from("abstract-value"));
    let abstract_put_invocation = test_invocation(vec![
        abstract_map.clone(),
        abstract_key.clone(),
        abstract_value.clone(),
    ]);
    let abstract_get_invocation = test_invocation(vec![abstract_map, abstract_key]);
    let weak_map = DexValue::Reference(std::rc::Rc::from("weak-map"));
    let weak_key = DexValue::Reference(std::rc::Rc::from("weak-key"));
    let weak_value = DexValue::Reference(std::rc::Rc::from("weak-value"));
    let weak_put_invocation =
        test_invocation(vec![weak_map.clone(), weak_key.clone(), weak_value.clone()]);
    let weak_get_invocation = test_invocation(vec![weak_map.clone(), weak_key.clone()]);
    let weak_empty_invocation = test_invocation(vec![weak_map]);

    assert_eq!(
        program.invoke_builtin_method(&map_size, Some(&size_invocation)),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&get, Some(&get_invocation)),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    assert_eq!(
        program.invoke_builtin_method(&contains_key, Some(&get_invocation)),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&put, Some(&put_invocation)),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    assert_eq!(
        program.invoke_builtin_method(&get, Some(&get_invocation)),
        Some(DexExecutionResult::Value(value.clone()))
    );
    assert_eq!(
        program.invoke_builtin_method(&contains_key, Some(&get_invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&map_size, Some(&size_invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&hash_map_size, Some(&size_invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&put, Some(&replace_invocation)),
        Some(DexExecutionResult::Value(value))
    );
    assert_eq!(
        program.invoke_builtin_method(&map_put, Some(&linked_put_invocation)),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    assert_eq!(
        program.invoke_builtin_method(&get, Some(&linked_get_invocation)),
        Some(DexExecutionResult::Value(linked_value.clone()))
    );
    assert_eq!(
        program.invoke_builtin_method(&map_size, Some(&linked_size_invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &hash_map_copy_constructor,
            Some(&copy_constructor_invocation),
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&get, Some(&copied_get_invocation)),
        Some(DexExecutionResult::Value(linked_value.clone()))
    );
    assert_eq!(
        program.invoke_builtin_method(&map_size, Some(&copied_size_invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &linked_hash_map_copy_constructor,
            Some(&linked_copy_constructor_invocation),
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&get, Some(&copied_linked_get_invocation)),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("replacement")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&map_size, Some(&copied_linked_size_invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&hash_map_put_all, Some(&put_all_invocation)),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&get, Some(&put_all_get_invocation)),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("replacement")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&map_size, Some(&put_all_size_invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&hash_map_remove, Some(&put_all_remove_invocation)),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("replacement")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&map_size, Some(&put_all_size_invocation)),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&hash_map_clear, Some(&clear_invocation)),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&map_size, Some(&size_invocation)),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&get, Some(&get_invocation)),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    assert_eq!(
        program.invoke_builtin_method(&abstract_map_put, Some(&abstract_put_invocation)),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    assert_eq!(
        program.invoke_builtin_method(&get, Some(&abstract_get_invocation)),
        Some(DexExecutionResult::Value(abstract_value))
    );
    assert_eq!(
        program.invoke_builtin_method(&weak_hash_map_is_empty, Some(&weak_empty_invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&weak_hash_map_put, Some(&weak_put_invocation)),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    assert_eq!(
        program.invoke_builtin_method(&weak_hash_map_is_empty, Some(&weak_empty_invocation)),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&weak_hash_map_get, Some(&weak_get_invocation)),
        Some(DexExecutionResult::Value(weak_value))
    );

    let Some(DexExecutionResult::Value(key_set)) =
        program.invoke_builtin_method(&weak_hash_map_key_set, Some(&weak_empty_invocation))
    else {
        panic!("WeakHashMap.keySet should return a set");
    };
    let Some(DexExecutionResult::Value(iterator)) =
        program.invoke_builtin_method(&set_iterator, Some(&test_invocation(vec![key_set])))
    else {
        panic!("WeakHashMap keySet should return an iterator");
    };
    assert_eq!(
        program.invoke_builtin_method(&iterator_next, Some(&test_invocation(vec![iterator]))),
        Some(DexExecutionResult::Value(weak_key))
    );
}

#[test]
fn builtin_map_entry_set_iterates_stored_entries() {
    let program = DexProgram::new(Vec::new());
    let map_put = DexMethodId {
        class_descriptor: "Ljava/util/Map;".to_owned(),
        name: "put".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LLL".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: vec![
                "Ljava/lang/Object;".to_owned(),
                "Ljava/lang/Object;".to_owned(),
            ],
        },
    };
    let entry_set = DexMethodId {
        class_descriptor: "Ljava/util/Map;".to_owned(),
        name: "entrySet".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/util/Set;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let hash_map_entry_set = DexMethodId {
        class_descriptor: "Ljava/util/HashMap;".to_owned(),
        name: "entrySet".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/util/Set;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let hash_map_values = DexMethodId {
        class_descriptor: "Ljava/util/HashMap;".to_owned(),
        name: "values".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/util/Collection;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let collection_iterator = DexMethodId {
        class_descriptor: "Ljava/util/Collection;".to_owned(),
        name: "iterator".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/util/Iterator;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set_iterator = DexMethodId {
        class_descriptor: "Ljava/util/Set;".to_owned(),
        name: "iterator".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/util/Iterator;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set_size = DexMethodId {
        class_descriptor: "Ljava/util/Set;".to_owned(),
        name: "size".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let has_next = DexMethodId {
        class_descriptor: "Ljava/util/Iterator;".to_owned(),
        name: "hasNext".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let next = DexMethodId {
        class_descriptor: "Ljava/util/Iterator;".to_owned(),
        name: "next".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get_key = DexMethodId {
        class_descriptor: "Ljava/util/Map$Entry;".to_owned(),
        name: "getKey".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get_value = DexMethodId {
        class_descriptor: "Ljava/util/Map$Entry;".to_owned(),
        name: "getValue".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let map = DexValue::Reference(std::rc::Rc::from("map"));
    let key = DexValue::String(std::rc::Rc::from("isCompressed"));
    let value = DexValue::Reference(std::rc::Rc::from("Ljava/lang/Boolean;:true"));
    let put_invocation = test_invocation(vec![map.clone(), key.clone(), value.clone()]);
    let entry_set_invocation = test_invocation(vec![map]);
    let set = DexValue::Reference(std::rc::Rc::from(
        "Ljava/util/Set;:mapEntrySet:reference:map",
    ));
    let iterator = DexValue::Reference(std::rc::Rc::from("Ljava/util/Iterator;:iterator:1"));
    let entry = DexValue::Reference(std::rc::Rc::from("Ljava/util/Map$Entry;:iterator:1:0"));
    let set_iterator_invocation = test_invocation(vec![set.clone()]);
    let iterator_invocation = test_invocation(vec![iterator.clone()]);
    let entry_invocation = test_invocation(vec![entry.clone()]);

    assert_eq!(
        program.invoke_builtin_method(&map_put, Some(&put_invocation)),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    assert_eq!(
        program.invoke_builtin_method(&entry_set, Some(&entry_set_invocation)),
        Some(DexExecutionResult::Value(set.clone()))
    );
    assert_eq!(
        program.invoke_builtin_method(&hash_map_entry_set, Some(&entry_set_invocation)),
        Some(DexExecutionResult::Value(set.clone()))
    );
    assert_eq!(
        program.invoke_builtin_method(&set_size, Some(&set_iterator_invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&set_iterator, Some(&set_iterator_invocation)),
        Some(DexExecutionResult::Value(iterator.clone()))
    );
    assert_eq!(
        program.invoke_builtin_method(&has_next, Some(&iterator_invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&next, Some(&iterator_invocation)),
        Some(DexExecutionResult::Value(entry))
    );
    assert_eq!(
        program.invoke_builtin_method(&get_key, Some(&entry_invocation)),
        Some(DexExecutionResult::Value(key))
    );
    assert_eq!(
        program.invoke_builtin_method(&get_value, Some(&entry_invocation)),
        Some(DexExecutionResult::Value(value.clone()))
    );
    assert_eq!(
        program.invoke_builtin_method(&has_next, Some(&iterator_invocation)),
        Some(DexExecutionResult::Int(0))
    );

    let Some(DexExecutionResult::Value(values_collection)) =
        program.invoke_builtin_method(&hash_map_values, Some(&entry_set_invocation))
    else {
        panic!("values should return a collection");
    };
    let Some(DexExecutionResult::Value(values_iterator)) = program.invoke_builtin_method(
        &collection_iterator,
        Some(&test_invocation(vec![values_collection])),
    ) else {
        panic!("collection iterator should return an iterator");
    };
    let values_iterator_invocation = test_invocation(vec![values_iterator]);

    assert_eq!(
        program.invoke_builtin_method(&has_next, Some(&values_iterator_invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&next, Some(&values_iterator_invocation)),
        Some(DexExecutionResult::Value(value))
    );
}

#[test]
fn builtin_collections_empty_map_returns_empty_map() {
    let program = DexProgram::new(Vec::new());
    let empty_map = DexMethodId {
        class_descriptor: "Ljava/util/Collections;".to_owned(),
        name: "emptyMap".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/util/Map;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let map_size = DexMethodId {
        class_descriptor: "Ljava/util/Map;".to_owned(),
        name: "size".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let result = program.invoke_builtin_method(&empty_map, None);
    let Some(DexExecutionResult::Value(map)) = result else {
        panic!("emptyMap did not return a map reference");
    };

    assert_eq!(
        program.invoke_builtin_method(&map_size, Some(&test_invocation(vec![map]))),
        Some(DexExecutionResult::Int(0))
    );
}
