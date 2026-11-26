use super::super::*;
use super::helpers::*;

#[test]
fn java_collection_interface_pre_dispatch_includes_set() {
    let method = DexMethodId {
        class_descriptor: "Ljava/util/Set;".to_owned(),
        name: "iterator".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/util/Iterator;".to_owned(),
            parameters: Vec::new(),
        },
    };

    assert!(java_collection_interface_method(&method));
}

#[test]
fn java_collection_interface_pre_dispatch_includes_iterator() {
    let method = DexMethodId {
        class_descriptor: "Ljava/util/Iterator;".to_owned(),
        name: "hasNext".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };

    assert!(java_collection_interface_method(&method));
}

#[test]
fn builtin_set_remove_removes_matching_value() {
    let program = DexProgram::new(Vec::new());
    let remove = DexMethodId {
        class_descriptor: "Ljava/util/Set;".to_owned(),
        name: "remove".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let set = program.allocate_object("Ljava/util/HashSet;".to_owned());
    let keep = DexValue::String(std::rc::Rc::from("keep"));
    let remove_value = DexValue::String(std::rc::Rc::from("remove"));
    program.runtime.borrow_mut().list_entries.insert(
        map_identity_key(&set),
        vec![keep.clone(), remove_value.clone()],
    );

    assert_eq!(
        program.invoke_builtin_method(
            &remove,
            Some(&test_invocation(vec![set.clone(), remove_value]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .list_entries
            .get(&map_identity_key(&set)),
        Some(&vec![keep])
    );
    assert_eq!(
        program.invoke_builtin_method(
            &remove,
            Some(&test_invocation(vec![
                set,
                DexValue::String(std::rc::Rc::from("missing")),
            ]))
        ),
        Some(DexExecutionResult::Int(0))
    );
}

#[test]
fn builtin_set_add_returns_false_for_existing_value() {
    let program = DexProgram::new(Vec::new());
    let add = DexMethodId {
        class_descriptor: "Ljava/util/Set;".to_owned(),
        name: "add".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let size = DexMethodId {
        class_descriptor: "Ljava/util/Set;".to_owned(),
        name: "size".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set = program.allocate_object("Ljava/util/LinkedHashSet;".to_owned());
    let value = DexValue::Reference(std::rc::Rc::from("value"));

    assert_eq!(
        program.invoke_builtin_method(
            &add,
            Some(&test_invocation(vec![set.clone(), value.clone()]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &add,
            Some(&test_invocation(vec![set.clone(), value.clone()]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&size, Some(&test_invocation(vec![set.clone()]))),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .list_entries
            .get(&map_identity_key(&set)),
        Some(&vec![value])
    );
}

#[test]
fn java_set_membership_uses_object_equals() {
    let dex_file = crate::dex::parse_dex_file("classes.dex", always_equal_class_dex())
        .expect("DEX should parse");
    let program = DexProgram::new(vec![dex_file]);
    let add = DexMethodId {
        class_descriptor: "Ljava/util/HashSet;".to_owned(),
        name: "add".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let contains = DexMethodId {
        class_descriptor: "Ljava/util/HashSet;".to_owned(),
        name: "contains".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let remove = DexMethodId {
        class_descriptor: "Ljava/util/HashSet;".to_owned(),
        name: "remove".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let set = program.allocate_object("Ljava/util/HashSet;".to_owned());
    let first = program.allocate_object("Lcom/example/Equal;".to_owned());
    let second = program.allocate_object("Lcom/example/Equal;".to_owned());

    assert_eq!(
        program
            .invoke_java_collection_equality_method(
                &add,
                &test_invocation(vec![set.clone(), first.clone()]),
                0,
            )
            .expect("Set.add should run"),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program
            .invoke_java_collection_equality_method(
                &add,
                &test_invocation(vec![set.clone(), second.clone()]),
                0,
            )
            .expect("Set.add should run"),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program
            .invoke_java_collection_equality_method(
                &contains,
                &test_invocation(vec![set.clone(), second.clone()]),
                0,
            )
            .expect("Set.contains should run"),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program
            .invoke_java_collection_equality_method(
                &remove,
                &test_invocation(vec![set.clone(), second]),
                0,
            )
            .expect("Set.remove should run"),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .list_entries
            .get(&map_identity_key(&set)),
        Some(&Vec::new())
    );
}

#[test]
fn java_hash_map_keys_use_object_equals() {
    let dex_file = crate::dex::parse_dex_file("classes.dex", always_equal_class_dex())
        .expect("DEX should parse");
    let program = DexProgram::new(vec![dex_file]);
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
    let get = DexMethodId {
        class_descriptor: "Ljava/util/HashMap;".to_owned(),
        name: "get".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let contains_key = DexMethodId {
        class_descriptor: "Ljava/util/HashMap;".to_owned(),
        name: "containsKey".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let remove = DexMethodId {
        class_descriptor: "Ljava/util/HashMap;".to_owned(),
        name: "remove".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let map = program.allocate_object("Ljava/util/HashMap;".to_owned());
    let first = program.allocate_object("Lcom/example/Equal;".to_owned());
    let second = program.allocate_object("Lcom/example/Equal;".to_owned());

    assert_eq!(
        program
            .invoke_java_collection_equality_method(
                &put,
                &test_invocation(vec![
                    map.clone(),
                    first.clone(),
                    DexValue::String(std::rc::Rc::from("value")),
                ]),
                0,
            )
            .expect("HashMap.put should run"),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    assert_eq!(
        program
            .invoke_java_collection_equality_method(
                &contains_key,
                &test_invocation(vec![map.clone(), second.clone()]),
                0,
            )
            .expect("HashMap.containsKey should run"),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program
            .invoke_java_collection_equality_method(
                &get,
                &test_invocation(vec![map.clone(), second.clone()]),
                0,
            )
            .expect("HashMap.get should run"),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("value")
        )))
    );
    assert_eq!(
        program
            .invoke_java_collection_equality_method(
                &put,
                &test_invocation(vec![
                    map.clone(),
                    second.clone(),
                    DexValue::String(std::rc::Rc::from("updated")),
                ]),
                0,
            )
            .expect("HashMap.put should run"),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("value")
        )))
    );
    assert_eq!(
        program
            .invoke_java_collection_equality_method(
                &remove,
                &test_invocation(vec![map.clone(), second]),
                0,
            )
            .expect("HashMap.remove should run"),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("updated")
        )))
    );
    assert!(program
        .runtime
        .borrow()
        .map_entries
        .keys()
        .all(|(map_key, _)| map_key != &map_identity_key(&map)));
}

#[test]
fn java_hash_map_equals_uses_object_equals_for_keys() {
    let dex_file = crate::dex::parse_dex_file("classes.dex", always_equal_class_dex())
        .expect("DEX should parse");
    let program = DexProgram::new(vec![dex_file]);
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
    let left = program.allocate_object("Ljava/util/HashMap;".to_owned());
    let right = program.allocate_object("Ljava/util/HashMap;".to_owned());
    let first_key = program.allocate_object("Lcom/example/Equal;".to_owned());
    let second_key = program.allocate_object("Lcom/example/Equal;".to_owned());
    let value = DexValue::String(std::rc::Rc::from("value"));

    program
        .invoke_java_collection_equality_method(
            &put,
            &test_invocation(vec![left.clone(), first_key, value.clone()]),
            0,
        )
        .expect("left HashMap.put should run");
    program
        .invoke_java_collection_equality_method(
            &put,
            &test_invocation(vec![right.clone(), second_key, value]),
            0,
        )
        .expect("right HashMap.put should run");

    assert_eq!(
        program
            .runtime_object_equals(&test_invocation(vec![left, right]), 0)
            .expect("HashMap.equals should run"),
        1
    );
}

#[test]
fn java_identity_hash_map_keeps_identity_key_semantics() {
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
    let map = program.allocate_object("Ljava/util/IdentityHashMap;".to_owned());
    let key = DexValue::Reference(std::rc::Rc::from("key"));

    assert_eq!(
        program
            .invoke_java_collection_equality_method(
                &get,
                &test_invocation(vec![map.clone(), key]),
                0,
            )
            .expect("IdentityHashMap should be checked"),
        None
    );
}

#[test]
fn java_identity_hash_map_equals_keeps_identity_key_semantics() {
    let dex_file = crate::dex::parse_dex_file("classes.dex", always_equal_class_dex())
        .expect("DEX should parse");
    let program = DexProgram::new(vec![dex_file]);
    let put = DexMethodId {
        class_descriptor: "Ljava/util/IdentityHashMap;".to_owned(),
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
    let left = program.allocate_object("Ljava/util/IdentityHashMap;".to_owned());
    let right = program.allocate_object("Ljava/util/IdentityHashMap;".to_owned());
    let first_key = program.allocate_object("Lcom/example/Equal;".to_owned());
    let second_key = program.allocate_object("Lcom/example/Equal;".to_owned());
    let value = DexValue::String(std::rc::Rc::from("value"));

    assert_eq!(
        program.invoke_builtin_method(
            &put,
            Some(&test_invocation(vec![
                left.clone(),
                first_key,
                value.clone()
            ]))
        ),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &put,
            Some(&test_invocation(vec![right.clone(), second_key, value]))
        ),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    assert_eq!(
        program
            .runtime_object_equals(&test_invocation(vec![left, right]), 0)
            .expect("IdentityHashMap.equals should run"),
        0
    );
}

fn always_equal_class_dex() -> Vec<u8> {
    const DEX_HEADER_SIZE: usize = 0x70;
    const DEX_HEADER_SIZE_U32: u32 = 0x70;
    const DEX_ENDIAN_CONSTANT: u32 = 0x1234_5678;
    const STRING_ID_SIZE: usize = 4;
    const TYPE_ID_SIZE: usize = 4;
    const PROTO_ID_SIZE: usize = 12;
    const METHOD_ID_SIZE: usize = 8;
    const CLASS_DEF_SIZE: usize = 32;

    let string_ids_off = DEX_HEADER_SIZE;
    let type_ids_off = string_ids_off + STRING_ID_SIZE * 5;
    let proto_ids_off = type_ids_off + TYPE_ID_SIZE * 3;
    let method_ids_off = proto_ids_off + PROTO_ID_SIZE;
    let class_defs_off = method_ids_off + METHOD_ID_SIZE;
    let data_off = class_defs_off + CLASS_DEF_SIZE;
    let mut data = vec![0; data_off];
    let strings = [
        "Lcom/example/Equal;",
        "Ljava/lang/Object;",
        "Z",
        "equals",
        "ZL",
    ];

    let parameters_off = data.len() as u32;
    data.extend_from_slice(&1u32.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&0u16.to_le_bytes());

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

    data.extend_from_slice(&2u16.to_le_bytes());
    data.extend_from_slice(&2u16.to_le_bytes());
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&2u32.to_le_bytes());
    data.extend_from_slice(&0x1012u16.to_le_bytes());
    data.extend_from_slice(&0x000fu16.to_le_bytes());

    let file_size = data.len() as u32;
    data[0..8].copy_from_slice(b"dex\n035\0");
    write_test_u32(&mut data, 32, file_size);
    write_test_u32(&mut data, 36, DEX_HEADER_SIZE_U32);
    write_test_u32(&mut data, 40, DEX_ENDIAN_CONSTANT);
    write_test_u32(&mut data, 56, 5);
    write_test_u32(&mut data, 60, string_ids_off as u32);
    write_test_u32(&mut data, 64, 3);
    write_test_u32(&mut data, 68, type_ids_off as u32);
    write_test_u32(&mut data, 72, 1);
    write_test_u32(&mut data, 76, proto_ids_off as u32);
    write_test_u32(&mut data, 88, 1);
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

    write_test_u32(&mut data, proto_ids_off, 4);
    write_test_u32(&mut data, proto_ids_off + 4, 2);
    write_test_u32(&mut data, proto_ids_off + 8, parameters_off);

    write_test_u16(&mut data, method_ids_off, 0);
    write_test_u16(&mut data, method_ids_off + 2, 0);
    write_test_u32(&mut data, method_ids_off + 4, 3);

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

#[test]
fn builtin_hash_set_capacity_constructor_initializes_empty_set() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Ljava/util/HashSet;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let size = DexMethodId {
        class_descriptor: "Ljava/util/HashSet;".to_owned(),
        name: "size".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set = program.allocate_object("Ljava/util/HashSet;".to_owned());

    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![set.clone(), DexValue::Int(8)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&size, Some(&test_invocation(vec![set.clone()]))),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .list_entries
            .get(&map_identity_key(&set)),
        Some(&Vec::new())
    );
    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![set, DexValue::Int(-1)]))
        ),
        None
    );
}

#[test]
fn builtin_collections_singleton_set_stores_one_value() {
    let program = DexProgram::new(Vec::new());
    let singleton_set = DexMethodId {
        class_descriptor: "Ljava/util/Collections;".to_owned(),
        name: "singleton".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/util/Set;".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
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
    let set_iterator = DexMethodId {
        class_descriptor: "Ljava/util/Set;".to_owned(),
        name: "iterator".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/util/Iterator;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set_contains = DexMethodId {
        class_descriptor: "Ljava/util/Set;".to_owned(),
        name: "contains".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
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
    let value = DexValue::Reference(std::rc::Rc::from("fragment"));
    let result =
        program.invoke_builtin_method(&singleton_set, Some(&test_invocation(vec![value.clone()])));
    let Some(DexExecutionResult::Value(set)) = result else {
        panic!("singleton did not return a set reference");
    };

    assert_eq!(
        program.invoke_builtin_method(&set_size, Some(&test_invocation(vec![set.clone()]))),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_contains,
            Some(&test_invocation(vec![set.clone(), value.clone()]))
        ),
        Some(DexExecutionResult::Int(1))
    );

    let Some(DexExecutionResult::Value(iterator)) =
        program.invoke_builtin_method(&set_iterator, Some(&test_invocation(vec![set])))
    else {
        panic!("singleton set did not return an iterator");
    };
    let iterator_invocation = test_invocation(vec![iterator]);

    assert_eq!(
        program.invoke_builtin_method(&has_next, Some(&iterator_invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&next, Some(&iterator_invocation)),
        Some(DexExecutionResult::Value(value))
    );
}

#[test]
fn builtin_collections_new_set_from_map_returns_empty_set() {
    let program = DexProgram::new(Vec::new());
    let new_set_from_map = DexMethodId {
        class_descriptor: "Ljava/util/Collections;".to_owned(),
        name: "newSetFromMap".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/util/Set;".to_owned(),
            parameters: vec!["Ljava/util/Map;".to_owned()],
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
    let backing_map = DexValue::Object(DexObjectRef {
        id: 1,
        class_descriptor: std::rc::Rc::from("Ljava/util/IdentityHashMap;"),
    });
    let result =
        program.invoke_builtin_method(&new_set_from_map, Some(&test_invocation(vec![backing_map])));
    let Some(DexExecutionResult::Value(set)) = result else {
        panic!("newSetFromMap did not return a set reference");
    };

    assert_eq!(
        program.invoke_builtin_method(&set_size, Some(&test_invocation(vec![set]))),
        Some(DexExecutionResult::Int(0))
    );
}

#[test]
fn runtime_object_equals_compares_managed_set_entries() {
    let program = DexProgram::new(Vec::new());
    let left = DexValue::Object(DexObjectRef {
        id: 1,
        class_descriptor: std::rc::Rc::from("Ljava/util/LinkedHashSet;"),
    });
    let right = DexValue::Object(DexObjectRef {
        id: 2,
        class_descriptor: std::rc::Rc::from("Ljava/util/LinkedHashSet;"),
    });

    assert_eq!(
        program
            .runtime_object_equals(&test_invocation(vec![left.clone(), right.clone()]), 0)
            .expect("empty set equality should run"),
        1
    );

    program.runtime.borrow_mut().list_entries.insert(
        map_identity_key(&left),
        vec![DexValue::String(std::rc::Rc::from("index"))],
    );

    assert_eq!(
        program
            .runtime_object_equals(&test_invocation(vec![left.clone(), right.clone()]), 0)
            .expect("set equality should run"),
        0
    );

    program.runtime.borrow_mut().list_entries.insert(
        map_identity_key(&right),
        vec![DexValue::String(std::rc::Rc::from("index"))],
    );

    assert_eq!(
        program
            .runtime_object_equals(&test_invocation(vec![left, right]), 0)
            .expect("set equality should run"),
        1
    );
}

#[test]
fn builtin_linked_hash_set_constructor_initializes_empty_storage() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Ljava/util/LinkedHashSet;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set = program.allocate_object("Ljava/util/LinkedHashSet;".to_owned());

    assert_eq!(
        program.invoke_builtin_method(&constructor, Some(&test_invocation(vec![set.clone()]))),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .list_entries
            .get(&map_identity_key(&set)),
        Some(&Vec::new())
    );
}

#[test]
fn java_linked_hash_set_collection_constructor_copies_values() {
    let dex_file = crate::dex::parse_dex_file("classes.dex", always_equal_class_dex())
        .expect("DEX should parse");
    let program = DexProgram::new(vec![dex_file]);
    let constructor = DexMethodId {
        class_descriptor: "Ljava/util/LinkedHashSet;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/util/Collection;".to_owned()],
        },
    };
    let size = DexMethodId {
        class_descriptor: "Ljava/util/Set;".to_owned(),
        name: "size".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let iterator_method = DexMethodId {
        class_descriptor: "Ljava/util/Set;".to_owned(),
        name: "iterator".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/util/Iterator;".to_owned(),
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
    let source = program.allocate_object("Ljava/util/ArrayList;".to_owned());
    let target = program.allocate_object("Ljava/util/LinkedHashSet;".to_owned());
    let first = program.allocate_object("Lcom/example/Equal;".to_owned());
    let duplicate = program.allocate_object("Lcom/example/Equal;".to_owned());
    let second = DexValue::String(std::rc::Rc::from("second"));

    program.runtime.borrow_mut().list_entries.insert(
        map_identity_key(&source),
        vec![first.clone(), duplicate, second.clone()],
    );

    assert_eq!(
        program
            .invoke_java_collection_equality_method(
                &constructor,
                &test_invocation(vec![target.clone(), source]),
                0,
            )
            .expect("LinkedHashSet(Collection) should run"),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&size, Some(&test_invocation(vec![target.clone()]))),
        Some(DexExecutionResult::Int(2))
    );

    let Some(DexExecutionResult::Value(iterator)) =
        program.invoke_builtin_method(&iterator_method, Some(&test_invocation(vec![target])))
    else {
        panic!("LinkedHashSet(Collection) did not return an iterator");
    };
    let iterator_invocation = test_invocation(vec![iterator]);

    assert_eq!(
        program.invoke_builtin_method(&has_next, Some(&iterator_invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&next, Some(&iterator_invocation)),
        Some(DexExecutionResult::Value(first))
    );
    assert_eq!(
        program.invoke_builtin_method(&next, Some(&iterator_invocation)),
        Some(DexExecutionResult::Value(second))
    );
    assert_eq!(
        program.invoke_builtin_method(&has_next, Some(&iterator_invocation)),
        Some(DexExecutionResult::Int(0))
    );
}

#[test]
fn builtin_collection_add_feeds_set_iteration() {
    let program = DexProgram::new(Vec::new());
    let add = DexMethodId {
        class_descriptor: "Ljava/util/Collection;".to_owned(),
        name: "add".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let abstract_collection_add = DexMethodId {
        class_descriptor: "Ljava/util/AbstractCollection;".to_owned(),
        name: "add".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let copy_on_write_add = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/CopyOnWriteArraySet;".to_owned(),
        name: "add".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let size = DexMethodId {
        class_descriptor: "Ljava/util/Set;".to_owned(),
        name: "size".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let iterator_method = DexMethodId {
        class_descriptor: "Ljava/util/Set;".to_owned(),
        name: "iterator".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/util/Iterator;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let copy_on_write_iterator_method = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/CopyOnWriteArraySet;".to_owned(),
        name: "iterator".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/util/Iterator;".to_owned(),
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
    let set = DexValue::Reference(std::rc::Rc::from("set"));
    let value = DexValue::String(std::rc::Rc::from("worker"));
    let add_invocation = test_invocation(vec![set.clone(), value.clone()]);
    let set_invocation = test_invocation(vec![set]);
    let abstract_set = DexValue::Reference(std::rc::Rc::from("abstract-set"));
    let abstract_value = DexValue::String(std::rc::Rc::from("abstract-worker"));
    let abstract_add_invocation =
        test_invocation(vec![abstract_set.clone(), abstract_value.clone()]);
    let abstract_set_invocation = test_invocation(vec![abstract_set]);
    let iterator = DexValue::Reference(std::rc::Rc::from("Ljava/util/Iterator;:iterator:1"));
    let iterator_invocation = test_invocation(vec![iterator.clone()]);
    let copy_on_write_set = DexValue::Object(DexObjectRef {
        id: 42,
        class_descriptor: std::rc::Rc::from("Ljava/util/concurrent/CopyOnWriteArraySet;"),
    });
    let copy_on_write_value = DexValue::String(std::rc::Rc::from("callback"));
    let copy_on_write_add_invocation =
        test_invocation(vec![copy_on_write_set.clone(), copy_on_write_value.clone()]);
    let copy_on_write_set_invocation = test_invocation(vec![copy_on_write_set.clone()]);
    let copy_on_write_iterator =
        DexValue::Reference(std::rc::Rc::from("Ljava/util/Iterator;:iterator:2"));
    let copy_on_write_iterator_invocation = test_invocation(vec![copy_on_write_iterator.clone()]);

    assert_eq!(
        program.invoke_builtin_method(&add, Some(&add_invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&size, Some(&set_invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&iterator_method, Some(&set_invocation)),
        Some(DexExecutionResult::Value(iterator.clone()))
    );
    assert_eq!(
        program.invoke_builtin_method(&has_next, Some(&iterator_invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&next, Some(&iterator_invocation)),
        Some(DexExecutionResult::Value(value))
    );
    assert_eq!(
        program.invoke_builtin_method(&has_next, Some(&iterator_invocation)),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&abstract_collection_add, Some(&abstract_add_invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&size, Some(&abstract_set_invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&copy_on_write_add, Some(&copy_on_write_add_invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&size, Some(&copy_on_write_set_invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &copy_on_write_iterator_method,
            Some(&copy_on_write_set_invocation)
        ),
        Some(DexExecutionResult::Value(copy_on_write_iterator.clone()))
    );
    assert_eq!(
        program.invoke_builtin_method(&has_next, Some(&copy_on_write_iterator_invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&next, Some(&copy_on_write_iterator_invocation)),
        Some(DexExecutionResult::Value(copy_on_write_value))
    );
}

#[test]
fn builtin_tree_set_constructor_stores_comparator_and_feeds_set_iteration() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Ljava/util/TreeSet;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/util/Comparator;".to_owned()],
        },
    };
    let size = DexMethodId {
        class_descriptor: "Ljava/util/TreeSet;".to_owned(),
        name: "size".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let iterator_method = DexMethodId {
        class_descriptor: "Ljava/util/TreeSet;".to_owned(),
        name: "iterator".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/util/Iterator;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let first = DexMethodId {
        class_descriptor: "Ljava/util/TreeSet;".to_owned(),
        name: "first".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
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
    let tree_set = DexValue::Object(DexObjectRef {
        id: 100,
        class_descriptor: std::rc::Rc::from("Ljava/util/TreeSet;"),
    });
    let comparator = DexValue::Object(DexObjectRef {
        id: 101,
        class_descriptor: std::rc::Rc::from("Lcom/example/Comparator;"),
    });
    let value = DexValue::String(std::rc::Rc::from("node"));
    let iterator = DexValue::Reference(std::rc::Rc::from("Ljava/util/Iterator;:iterator:1"));

    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![tree_set.clone(), comparator.clone()])),
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .tree_set_comparators
            .get(&map_identity_key(&tree_set)),
        Some(&comparator)
    );
    assert_eq!(
        program
            .tree_set_add_value(&test_invocation(vec![tree_set.clone(), value.clone()]), 0)
            .expect("TreeSet add should accept first value with a comparator"),
        1
    );
    assert_eq!(
        program.invoke_builtin_method(&size, Some(&test_invocation(vec![tree_set.clone()]))),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&first, Some(&test_invocation(vec![tree_set.clone()]))),
        Some(DexExecutionResult::Value(value.clone()))
    );
    assert_eq!(
        program.invoke_builtin_method(&iterator_method, Some(&test_invocation(vec![tree_set]))),
        Some(DexExecutionResult::Value(iterator.clone()))
    );
    assert_eq!(
        program.invoke_builtin_method(&has_next, Some(&test_invocation(vec![iterator.clone()]))),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&next, Some(&test_invocation(vec![iterator]))),
        Some(DexExecutionResult::Value(value))
    );
}

// cite: libcore TreeMap.java#compare
#[test]
fn tree_set_add_without_comparator_reports_missing_natural_ordering_for_unsupported_values() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Ljava/util/TreeSet;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let add = DexMethodId {
        class_descriptor: "Ljava/util/TreeSet;".to_owned(),
        name: "add".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let tree_set = DexValue::Object(DexObjectRef {
        id: 102,
        class_descriptor: std::rc::Rc::from("Ljava/util/TreeSet;"),
    });
    let first = DexValue::Object(DexObjectRef {
        id: 103,
        class_descriptor: std::rc::Rc::from("Lcom/example/NotBuiltinComparable;"),
    });
    let second = DexValue::Object(DexObjectRef {
        id: 104,
        class_descriptor: std::rc::Rc::from("Lcom/example/NotBuiltinComparable;"),
    });

    assert_eq!(
        program.invoke_builtin_method(&constructor, Some(&test_invocation(vec![tree_set.clone()]))),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &add,
            Some(&test_invocation(vec![
                tree_set.clone(),
                DexValue::String(std::rc::Rc::from("node")),
            ])),
        ),
        None
    );
    assert_eq!(
        program
            .tree_set_add_value(&test_invocation(vec![tree_set.clone(), first]), 0)
            .unwrap(),
        1
    );
    assert!(program
        .tree_set_add_value(&test_invocation(vec![tree_set, second]), 0)
        .is_err());
}

// cite: libcore ConcurrentSkipListSet.java#ConcurrentSkipListSet, add
// cite: libcore ConcurrentSkipListMap.java#cpr, doPut
#[test]
fn concurrent_skip_list_set_natural_ordering_keeps_sorted_unique_values_and_rejects_null() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/ConcurrentSkipListSet;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set = program.allocate_object("Ljava/util/concurrent/ConcurrentSkipListSet;".to_owned());
    let add = |value: &str| {
        program
            .tree_set_add_value(
                &test_invocation(vec![
                    set.clone(),
                    DexValue::String(std::rc::Rc::from(value)),
                ]),
                0,
            )
            .unwrap()
    };

    assert_eq!(
        program.invoke_builtin_method(&constructor, Some(&test_invocation(vec![set.clone()]))),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(add("pear"), 1);
    assert_eq!(add("apple"), 1);
    assert_eq!(add("pear"), 0);
    assert_eq!(add("mango"), 1);
    assert_eq!(
        program
            .runtime
            .borrow()
            .list_entries
            .get(&map_identity_key(&set))
            .cloned(),
        Some(vec![
            DexValue::String(std::rc::Rc::from("apple")),
            DexValue::String(std::rc::Rc::from("mango")),
            DexValue::String(std::rc::Rc::from("pear")),
        ])
    );
    for null in [DexValue::Null, DexValue::Int(0)] {
        assert!(program.sorted_set_add_rejects_null(&test_invocation(vec![set.clone(), null])));
    }
    assert!(!program.sorted_set_add_rejects_null(&test_invocation(vec![
        set,
        DexValue::String(std::rc::Rc::from("x"))
    ])));
}

// cite: libcore ConcurrentSkipListMap.java#doPut
// cite: libcore TreeMap.java#put
#[test]
fn concurrent_skip_list_set_rejects_null_even_with_a_comparator_but_tree_set_does_not() {
    let program = DexProgram::new(Vec::new());
    let comparator = program.allocate_object("Lcom/example/Order;".to_owned());
    let constructor = |class_descriptor: &str| DexMethodId {
        class_descriptor: class_descriptor.to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/util/Comparator;".to_owned()],
        },
    };
    let rejects_null = |class_descriptor: &str| {
        let set = program.allocate_object(class_descriptor.to_owned());
        assert_eq!(
            program.invoke_builtin_method(
                &constructor(class_descriptor),
                Some(&test_invocation(vec![set.clone(), comparator.clone()])),
            ),
            Some(DexExecutionResult::Void)
        );
        program.sorted_set_add_rejects_null(&test_invocation(vec![set, DexValue::Null]))
    };

    assert!(rejects_null("Ljava/util/concurrent/ConcurrentSkipListSet;"));
    assert!(!rejects_null("Ljava/util/TreeSet;"));

    let natural = program.allocate_object("Ljava/util/TreeSet;".to_owned());
    assert!(program.sorted_set_add_rejects_null(&test_invocation(vec![natural, DexValue::Null])));
}

#[test]
fn builtin_collection_remove_all_removes_matching_values() {
    let program = DexProgram::new(Vec::new());
    let add = DexMethodId {
        class_descriptor: "Ljava/util/Collection;".to_owned(),
        name: "add".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let singleton = DexMethodId {
        class_descriptor: "Ljava/util/Collections;".to_owned(),
        name: "singleton".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/util/Set;".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let remove_all = DexMethodId {
        class_descriptor: "Ljava/util/Collection;".to_owned(),
        name: "removeAll".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/util/Collection;".to_owned()],
        },
    };
    let add_all = DexMethodId {
        class_descriptor: "Ljava/util/ArrayList;".to_owned(),
        name: "addAll".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/util/Collection;".to_owned()],
        },
    };
    let size = DexMethodId {
        class_descriptor: "Ljava/util/Set;".to_owned(),
        name: "size".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let collection = DexValue::Reference(std::rc::Rc::from("collection"));
    let target = DexValue::Reference(std::rc::Rc::from("target"));
    let kept = DexValue::Reference(std::rc::Rc::from("kept"));
    let removed = DexValue::Reference(std::rc::Rc::from("removed"));

    assert_eq!(
        program
            .invoke_builtin_method(&add, Some(&test_invocation(vec![collection.clone(), kept])),),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &add,
            Some(&test_invocation(vec![collection.clone(), removed.clone()])),
        ),
        Some(DexExecutionResult::Int(1))
    );

    let Some(DexExecutionResult::Value(removed_collection)) =
        program.invoke_builtin_method(&singleton, Some(&test_invocation(vec![removed])))
    else {
        panic!("singleton did not return a set reference");
    };

    assert_eq!(
        program.invoke_builtin_method(
            &remove_all,
            Some(&test_invocation(vec![
                collection.clone(),
                removed_collection
            ])),
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&size, Some(&test_invocation(vec![collection.clone()]))),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program
            .invoke_java_collection_equality_method(
                &add_all,
                &test_invocation(vec![target.clone(), collection]),
                0,
            )
            .expect("ArrayList.addAll should run"),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&size, Some(&test_invocation(vec![target]))),
        Some(DexExecutionResult::Int(1))
    );
}

#[test]
fn libcore_abstract_set_remove_all_removes_matching_returns_false_for_none_and_throws_for_null() {
    let program = DexProgram::new(Vec::new());
    let add = DexMethodId {
        class_descriptor: "Ljava/util/Set;".to_owned(),
        name: "add".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let remove_all = DexMethodId {
        class_descriptor: "Ljava/util/AbstractSet;".to_owned(),
        name: "removeAll".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/util/Collection;".to_owned()],
        },
    };
    let set = program.allocate_object("Ljava/util/HashSet;".to_owned());
    let matching = program.allocate_object("Ljava/util/HashSet;".to_owned());
    let other = program.allocate_object("Ljava/util/HashSet;".to_owned());
    let empty = program.allocate_object("Ljava/util/HashSet;".to_owned());
    let first = DexValue::String(std::rc::Rc::from("first"));
    let second = DexValue::String(std::rc::Rc::from("second"));
    let third = DexValue::String(std::rc::Rc::from("third"));

    for (receiver, value) in [
        (&set, &first),
        (&set, &second),
        (&matching, &first),
        (&other, &third),
    ] {
        program
            .invoke_java_collection_equality_method(
                &add,
                &test_invocation(vec![receiver.clone(), value.clone()]),
                0,
            )
            .expect("Set.add should run");
    }
    let remove_all_result = |argument: DexValue| {
        program
            .invoke_java_collection_equality_method(
                &remove_all,
                &test_invocation(vec![set.clone(), argument]),
                0,
            )
            .expect("AbstractSet.removeAll should run")
    };

    assert_eq!(remove_all_result(other), Some(DexExecutionResult::Int(0)));
    assert_eq!(remove_all_result(empty), Some(DexExecutionResult::Int(0)));
    assert_eq!(
        remove_all_result(matching.clone()),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        remove_all_result(matching),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .list_entries
            .get(&map_identity_key(&set)),
        Some(&vec![second])
    );
    let Some(DexExecutionResult::Throw(exception)) = remove_all_result(DexValue::Null) else {
        panic!("AbstractSet.removeAll(null) should throw");
    };
    assert_eq!(
        value_class_descriptor(&exception),
        Some("Ljava/lang/NullPointerException;")
    );
}

#[test]
fn builtin_set_add_stores_value_for_later_size() {
    let program = DexProgram::new(Vec::new());
    let add = DexMethodId {
        class_descriptor: "Ljava/util/Set;".to_owned(),
        name: "add".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let size = DexMethodId {
        class_descriptor: "Ljava/util/Set;".to_owned(),
        name: "size".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set = DexValue::Reference(std::rc::Rc::from("set"));
    let value = DexValue::String(std::rc::Rc::from("tag"));
    let add_invocation = test_invocation(vec![set.clone(), value]);
    let size_invocation = test_invocation(vec![set]);

    assert_eq!(
        program.invoke_builtin_method(&add, Some(&add_invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&size, Some(&size_invocation)),
        Some(DexExecutionResult::Int(1))
    );
}

#[test]
fn builtin_set_add_all_adds_projected_collection_values_once() {
    let program = DexProgram::new(Vec::new());
    let singleton = DexMethodId {
        class_descriptor: "Ljava/util/Collections;".to_owned(),
        name: "singleton".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/util/Set;".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let add_all = DexMethodId {
        class_descriptor: "Ljava/util/Set;".to_owned(),
        name: "addAll".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/util/Collection;".to_owned()],
        },
    };
    let size = DexMethodId {
        class_descriptor: "Ljava/util/Set;".to_owned(),
        name: "size".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let target = program.allocate_object("Ljava/util/LinkedHashSet;".to_owned());
    let value = DexValue::String(std::rc::Rc::from("route"));
    let Some(DexExecutionResult::Value(source)) =
        program.invoke_builtin_method(&singleton, Some(&test_invocation(vec![value])))
    else {
        panic!("singleton did not return a set reference");
    };

    assert_eq!(
        program.invoke_builtin_method(
            &add_all,
            Some(&test_invocation(vec![target.clone(), source.clone()])),
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &add_all,
            Some(&test_invocation(vec![target.clone(), source])),
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&size, Some(&test_invocation(vec![target]))),
        Some(DexExecutionResult::Int(1))
    );
}

#[test]
fn builtin_appcompat_array_set_add_stores_value_for_later_size() {
    let program = DexProgram::new(Vec::new());
    let add = DexMethodId {
        class_descriptor: "Ls/e;".to_owned(),
        name: "add".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let size = DexMethodId {
        class_descriptor: "Ls/e;".to_owned(),
        name: "size".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set = DexValue::Reference(std::rc::Rc::from("Ls/e;.bootstrap"));
    let value = DexValue::Object(DexObjectRef {
        id: 1,
        class_descriptor: std::rc::Rc::from("Lg/F;"),
    });
    let add_invocation = test_invocation(vec![set.clone(), value]);
    let size_invocation = test_invocation(vec![set]);

    assert_eq!(
        program.invoke_builtin_method(&add, Some(&add_invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&size, Some(&size_invocation)),
        Some(DexExecutionResult::Int(1))
    );
}

#[test]
fn builtin_collection_is_empty_reads_collection_storage() {
    let program = DexProgram::new(Vec::new());
    let add = DexMethodId {
        class_descriptor: "Ljava/util/Collection;".to_owned(),
        name: "add".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let is_empty = DexMethodId {
        class_descriptor: "Ljava/util/Collection;".to_owned(),
        name: "isEmpty".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let size = DexMethodId {
        class_descriptor: "Ljava/util/Collection;".to_owned(),
        name: "size".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let abstract_remove = DexMethodId {
        class_descriptor: "Ljava/util/AbstractCollection;".to_owned(),
        name: "remove".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let collection = DexValue::Reference(std::rc::Rc::from("collection"));
    let collection_invocation = test_invocation(vec![collection.clone()]);
    let value = DexValue::String(std::rc::Rc::from("content-uri"));
    let add_invocation = test_invocation(vec![collection.clone(), value.clone()]);

    assert_eq!(
        program.invoke_builtin_method(&is_empty, Some(&collection_invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&add, Some(&add_invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&is_empty, Some(&collection_invocation)),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&size, Some(&collection_invocation)),
        Some(DexExecutionResult::Int(1))
    );

    let abstract_is_empty = DexMethodId {
        class_descriptor: "Ljava/util/AbstractCollection;".to_owned(),
        name: "isEmpty".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let abstract_size = DexMethodId {
        class_descriptor: "Ljava/util/AbstractCollection;".to_owned(),
        name: "size".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&abstract_is_empty, Some(&collection_invocation)),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&abstract_size, Some(&collection_invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &abstract_remove,
            Some(&test_invocation(vec![collection.clone(), value])),
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&abstract_is_empty, Some(&collection_invocation)),
        Some(DexExecutionResult::Int(1))
    );
}

#[test]
fn java_abstract_collection_add_all_copies_values() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/util/AbstractCollection;".to_owned(),
        name: "addAll".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/util/Collection;".to_owned()],
        },
    };
    let target = DexValue::Reference(std::rc::Rc::from("target"));
    let source = DexValue::Reference(std::rc::Rc::from("source"));
    let first = DexValue::String(std::rc::Rc::from("first"));
    let second = DexValue::String(std::rc::Rc::from("second"));

    program.runtime.borrow_mut().list_entries.insert(
        map_identity_key(&source),
        vec![first.clone(), second.clone()],
    );

    assert_eq!(
        program
            .invoke_java_collection_equality_method(
                &method,
                &test_invocation(vec![target.clone(), source]),
                0,
            )
            .expect("AbstractCollection.addAll should run"),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .list_entries
            .get(&map_identity_key(&target)),
        Some(&vec![first, second])
    );
}

#[test]
fn builtin_collection_retain_all_keeps_only_matching_values() {
    // cite: libcore AbstractCollection.java#retainAll
    let program = DexProgram::new(Vec::new());
    let add = DexMethodId {
        class_descriptor: "Ljava/util/Collection;".to_owned(),
        name: "add".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let singleton = DexMethodId {
        class_descriptor: "Ljava/util/Collections;".to_owned(),
        name: "singleton".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/util/Set;".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let retain_all = DexMethodId {
        class_descriptor: "Ljava/util/Collection;".to_owned(),
        name: "retainAll".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/util/Collection;".to_owned()],
        },
    };
    let size = DexMethodId {
        class_descriptor: "Ljava/util/Collection;".to_owned(),
        name: "size".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let collection = DexValue::Reference(std::rc::Rc::from("retain-collection"));
    let kept = DexValue::Reference(std::rc::Rc::from("kept"));
    let removed = DexValue::Reference(std::rc::Rc::from("removed"));

    assert_eq!(
        program.invoke_builtin_method(
            &add,
            Some(&test_invocation(vec![collection.clone(), kept.clone()]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &add,
            Some(&test_invocation(vec![collection.clone(), removed]))
        ),
        Some(DexExecutionResult::Int(1))
    );

    let Some(DexExecutionResult::Value(retained_set)) =
        program.invoke_builtin_method(&singleton, Some(&test_invocation(vec![kept])))
    else {
        panic!("Collections.singleton should return a set");
    };

    assert_eq!(
        program.invoke_builtin_method(
            &retain_all,
            Some(&test_invocation(vec![collection.clone(), retained_set]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&size, Some(&test_invocation(vec![collection]))),
        Some(DexExecutionResult::Int(1))
    );
}

#[test]
fn builtin_list_last_index_of_finds_the_final_occurrence() {
    // cite: libcore AbstractList.java#lastIndexOf
    let program = DexProgram::new(Vec::new());
    let add = DexMethodId {
        class_descriptor: "Ljava/util/Collection;".to_owned(),
        name: "add".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let last_index_of = DexMethodId {
        class_descriptor: "Ljava/util/List;".to_owned(),
        name: "lastIndexOf".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "IL".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let list = DexValue::Reference(std::rc::Rc::from("last-index-of-list"));
    let target = DexValue::String(std::rc::Rc::from("dup"));
    let other = DexValue::String(std::rc::Rc::from("other"));

    for value in [target.clone(), other, target.clone()] {
        assert_eq!(
            program.invoke_builtin_method(&add, Some(&test_invocation(vec![list.clone(), value]))),
            Some(DexExecutionResult::Int(1))
        );
    }

    assert_eq!(
        program.invoke_builtin_method(&last_index_of, Some(&test_invocation(vec![list, target]))),
        Some(DexExecutionResult::Int(2))
    );
}

#[test]
fn libcore_map_contains_value_follows_hash_map_and_concurrent_hash_map_java() {
    // cite: libcore ConcurrentHashMap.java#contains, containsValue
    // cite: libcore HashMap.java#containsValue
    let program = DexProgram::new(Vec::new());
    let method = |class_descriptor: &str, name: &str| DexMethodId {
        class_descriptor: class_descriptor.to_owned(),
        name: name.to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let fill = |class_descriptor: &str| {
        let map = program.allocate_object(class_descriptor.to_owned());
        let map_key = map_identity_key(&map);
        for (key, value) in [
            ("k1", DexValue::String(std::rc::Rc::from("v"))),
            ("k2", DexValue::Null),
        ] {
            program
                .runtime
                .borrow_mut()
                .map_entries
                .insert((map_key.clone(), key.to_owned()), value);
        }
        map
    };
    let call = |method: &DexMethodId, map: &DexValue, needle: DexValue| {
        program
            .invoke_java_collection_equality_method(
                method,
                &test_invocation(vec![map.clone(), needle]),
                0,
            )
            .expect("containsValue should run")
    };
    let text = |value: &str| DexValue::String(std::rc::Rc::from(value));

    let hash_map = fill("Ljava/util/HashMap;");
    let contains_value = method("Ljava/util/HashMap;", "containsValue");
    assert_eq!(
        call(&contains_value, &hash_map, text("v")),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        call(&contains_value, &hash_map, text("w")),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        call(&contains_value, &hash_map, DexValue::Null),
        Some(DexExecutionResult::Int(1))
    );

    let concurrent = program.allocate_object("Ljava/util/concurrent/ConcurrentHashMap;".to_owned());
    program
        .runtime
        .borrow_mut()
        .map_entries
        .insert((map_identity_key(&concurrent), "k".to_owned()), text("v"));
    let contains = method("Ljava/util/concurrent/ConcurrentHashMap;", "contains");
    assert_eq!(
        call(&contains, &concurrent, text("v")),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        call(&contains, &concurrent, text("w")),
        Some(DexExecutionResult::Int(0))
    );
    let Some(DexExecutionResult::Throw(exception)) = call(&contains, &concurrent, DexValue::Null)
    else {
        panic!("ConcurrentHashMap.contains(null) should throw");
    };
    assert!(program.value_matches_type(&exception, "Ljava/lang/NullPointerException;"));
}
