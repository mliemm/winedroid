use super::super::*;
use super::helpers::*;

// cite: libcore ArrayDeque.java#offer, offerLast, addLast
#[test]
fn libcore_array_deque_offer_appends_tail_entry_and_returns_true() {
    let program = DexProgram::new(Vec::new());
    let offer = DexMethodId {
        class_descriptor: "Ljava/util/ArrayDeque;".to_owned(),
        name: "offer".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let deque = program.allocate_object("Ljava/util/ArrayDeque;".to_owned());
    let value = DexValue::String(std::rc::Rc::from("tail"));

    assert_eq!(
        program.invoke_builtin_method(
            &offer,
            Some(&test_invocation(vec![deque.clone(), value.clone()]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .list_entries
            .get(&map_identity_key(&deque))
            .cloned(),
        Some(vec![value])
    );
}

#[test]
fn builtin_list_iterator_at_supports_previous_and_next() {
    let program = DexProgram::new(Vec::new());
    let list_iterator = DexMethodId {
        class_descriptor: "Ljava/util/List;".to_owned(),
        name: "listIterator".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LI".to_owned(),
            return_type: "Ljava/util/ListIterator;".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let has_previous = DexMethodId {
        class_descriptor: "Ljava/util/ListIterator;".to_owned(),
        name: "hasPrevious".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let previous = DexMethodId {
        class_descriptor: "Ljava/util/ListIterator;".to_owned(),
        name: "previous".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let has_next = DexMethodId {
        class_descriptor: "Ljava/util/ListIterator;".to_owned(),
        name: "hasNext".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let next = DexMethodId {
        class_descriptor: "Ljava/util/ListIterator;".to_owned(),
        name: "next".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let list = program.allocate_object("Ljava/util/ArrayList;".to_owned());
    let first = DexValue::String(std::rc::Rc::from("first"));
    let second = DexValue::String(std::rc::Rc::from("second"));

    program
        .runtime
        .borrow_mut()
        .list_entries
        .insert(map_identity_key(&list), vec![first.clone(), second.clone()]);

    let Some(DexExecutionResult::Value(iterator)) = program.invoke_builtin_method(
        &list_iterator,
        Some(&test_invocation(vec![list, DexValue::Int(2)])),
    ) else {
        panic!("List.listIterator(int) did not return an iterator");
    };
    let iterator_invocation = test_invocation(vec![iterator]);

    assert_eq!(
        program.invoke_builtin_method(&has_previous, Some(&iterator_invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&previous, Some(&iterator_invocation)),
        Some(DexExecutionResult::Value(second))
    );
    assert_eq!(
        program.invoke_builtin_method(&previous, Some(&iterator_invocation)),
        Some(DexExecutionResult::Value(first.clone()))
    );
    assert_eq!(
        program.invoke_builtin_method(&has_previous, Some(&iterator_invocation)),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&has_next, Some(&iterator_invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&next, Some(&iterator_invocation)),
        Some(DexExecutionResult::Value(first))
    );
}

#[test]
fn builtin_iterator_remove_updates_backing_list() {
    let program = DexProgram::new(Vec::new());
    let iterator = DexMethodId {
        class_descriptor: "Ljava/util/ArrayList;".to_owned(),
        name: "iterator".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/util/Iterator;".to_owned(),
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
    let remove = DexMethodId {
        class_descriptor: "Ljava/util/Iterator;".to_owned(),
        name: "remove".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let list = program.allocate_object("Ljava/util/ArrayList;".to_owned());
    let list_key = map_identity_key(&list);
    let first = DexValue::String(std::rc::Rc::from("first"));
    let second = DexValue::String(std::rc::Rc::from("second"));

    program
        .runtime
        .borrow_mut()
        .list_entries
        .insert(list_key.clone(), vec![first.clone(), second.clone()]);

    let Some(DexExecutionResult::Value(iterator)) =
        program.invoke_builtin_method(&iterator, Some(&test_invocation(vec![list])))
    else {
        panic!("ArrayList.iterator() did not return an iterator");
    };
    let iterator_invocation = test_invocation(vec![iterator]);

    assert_eq!(
        program.invoke_builtin_method(&next, Some(&iterator_invocation)),
        Some(DexExecutionResult::Value(first))
    );
    assert_eq!(
        program.invoke_builtin_method(&remove, Some(&iterator_invocation)),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.runtime.borrow().list_entries.get(&list_key),
        Some(&vec![second])
    );
}

#[test]
fn builtin_abstract_list_iterator_uses_list_entries() {
    let program = DexProgram::new(Vec::new());
    let iterator = DexMethodId {
        class_descriptor: "Ljava/util/AbstractList;".to_owned(),
        name: "iterator".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/util/Iterator;".to_owned(),
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
    let list = program.allocate_object("Lexample/AppList;".to_owned());
    let first = DexValue::String(std::rc::Rc::from("first"));

    program
        .runtime
        .borrow_mut()
        .list_entries
        .insert(map_identity_key(&list), vec![first.clone()]);

    let Some(DexExecutionResult::Value(iterator)) =
        program.invoke_builtin_method(&iterator, Some(&test_invocation(vec![list])))
    else {
        panic!("AbstractList.iterator() did not return an iterator");
    };

    assert_eq!(
        program.invoke_builtin_method(&next, Some(&test_invocation(vec![iterator]))),
        Some(DexExecutionResult::Value(first))
    );
}

#[test]
fn builtin_array_list_constructor_returns_void() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Ljava/util/ArrayList;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let capacity_constructor = DexMethodId {
        class_descriptor: "Ljava/util/ArrayList;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let collection_constructor = DexMethodId {
        class_descriptor: "Ljava/util/ArrayList;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/util/Collection;".to_owned()],
        },
    };
    let abstract_list_constructor = DexMethodId {
        class_descriptor: "Ljava/util/AbstractList;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let copy_on_write_constructor = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/CopyOnWriteArrayList;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let add = DexMethodId {
        class_descriptor: "Ljava/util/ArrayList;".to_owned(),
        name: "add".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let copy_on_write_add = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/CopyOnWriteArrayList;".to_owned(),
        name: "add".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let copy_on_write_remove = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/CopyOnWriteArrayList;".to_owned(),
        name: "remove".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let copy_on_write_size = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/CopyOnWriteArrayList;".to_owned(),
        name: "size".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let copy_on_write_is_empty = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/CopyOnWriteArrayList;".to_owned(),
        name: "isEmpty".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let copy_on_write_iterator = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/CopyOnWriteArrayList;".to_owned(),
        name: "iterator".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/util/Iterator;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get = DexMethodId {
        class_descriptor: "Ljava/util/ArrayList;".to_owned(),
        name: "get".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LI".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let remove = DexMethodId {
        class_descriptor: "Ljava/util/ArrayList;".to_owned(),
        name: "remove".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LI".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let clear = DexMethodId {
        class_descriptor: "Ljava/util/ArrayList;".to_owned(),
        name: "clear".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let list_clear = DexMethodId {
        class_descriptor: "Ljava/util/List;".to_owned(),
        name: "clear".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let array_list_iterator = DexMethodId {
        class_descriptor: "Ljava/util/ArrayList;".to_owned(),
        name: "iterator".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/util/Iterator;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let array_list_size = DexMethodId {
        class_descriptor: "Ljava/util/ArrayList;".to_owned(),
        name: "size".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let array_list_is_empty = DexMethodId {
        class_descriptor: "Ljava/util/ArrayList;".to_owned(),
        name: "isEmpty".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let list_get = DexMethodId {
        class_descriptor: "Ljava/util/List;".to_owned(),
        name: "get".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LI".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let list_add = DexMethodId {
        class_descriptor: "Ljava/util/List;".to_owned(),
        name: "add".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let list_add_at = DexMethodId {
        class_descriptor: "Ljava/util/List;".to_owned(),
        name: "add".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VIL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned(), "Ljava/lang/Object;".to_owned()],
        },
    };
    let array_list_add_at = DexMethodId {
        class_descriptor: "Ljava/util/ArrayList;".to_owned(),
        name: "add".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VIL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned(), "Ljava/lang/Object;".to_owned()],
        },
    };
    let array_list_set = DexMethodId {
        class_descriptor: "Ljava/util/ArrayList;".to_owned(),
        name: "set".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LIL".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: vec!["I".to_owned(), "Ljava/lang/Object;".to_owned()],
        },
    };
    let list_contains = DexMethodId {
        class_descriptor: "Ljava/util/List;".to_owned(),
        name: "contains".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let array_list_contains = DexMethodId {
        class_descriptor: "Ljava/util/ArrayList;".to_owned(),
        name: "contains".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let list_size = DexMethodId {
        class_descriptor: "Ljava/util/List;".to_owned(),
        name: "size".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let list_is_empty = DexMethodId {
        class_descriptor: "Ljava/util/List;".to_owned(),
        name: "isEmpty".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let invocation = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from("list")),
        DexValue::Reference(std::rc::Rc::from("value")),
    ]);
    let list_invocation = test_invocation(vec![DexValue::Reference(std::rc::Rc::from("list"))]);
    let get_invocation = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from("list")),
        DexValue::Int(0),
    ]);
    let copy_on_write_invocation = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from("copy-on-write-list")),
        DexValue::Reference(std::rc::Rc::from("copy-on-write-value")),
    ]);
    let copy_on_write_size_invocation = test_invocation(vec![DexValue::Reference(
        std::rc::Rc::from("copy-on-write-list"),
    )]);
    let copied_list_invocation =
        test_invocation(vec![DexValue::Reference(std::rc::Rc::from("copied-list"))]);
    let indexed_list_invocation =
        test_invocation(vec![DexValue::Reference(std::rc::Rc::from("indexed-list"))]);
    let empty_list = DexValue::Reference(std::rc::Rc::from("empty-list"));
    let capacity_list = DexValue::Reference(std::rc::Rc::from("list"));
    let copy_on_write_list = DexValue::Reference(std::rc::Rc::from("copy-on-write-list"));

    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![empty_list.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .list_entries
            .get(&map_identity_key(&empty_list)),
        Some(&Vec::<DexValue>::new())
    );
    assert_eq!(
        program.invoke_builtin_method(
            &capacity_constructor,
            Some(&test_invocation(vec![
                capacity_list.clone(),
                DexValue::Int(2)
            ])),
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .list_entries
            .get(&map_identity_key(&capacity_list)),
        Some(&Vec::<DexValue>::new())
    );
    assert_eq!(
        program.invoke_builtin_method(&abstract_list_constructor, None),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &copy_on_write_constructor,
            Some(&test_invocation(vec![copy_on_write_list.clone()])),
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .list_entries
            .get(&map_identity_key(&copy_on_write_list)),
        Some(&Vec::<DexValue>::new())
    );
    assert_eq!(
        program.invoke_builtin_method(
            &copy_on_write_is_empty,
            Some(&copy_on_write_size_invocation)
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&list_is_empty, Some(&list_invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&array_list_is_empty, Some(&list_invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&array_list_contains, Some(&invocation)),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&list_add, Some(&invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&list_contains, Some(&invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&add, Some(&invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&array_list_contains, Some(&invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&copy_on_write_add, Some(&copy_on_write_invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&list_size, Some(&copy_on_write_size_invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&copy_on_write_size, Some(&copy_on_write_size_invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &copy_on_write_is_empty,
            Some(&copy_on_write_size_invocation)
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert!(matches!(
        program.invoke_builtin_method(
            &copy_on_write_iterator,
            Some(&copy_on_write_size_invocation)
        ),
        Some(DexExecutionResult::Value(DexValue::Reference(_)))
    ));
    assert_eq!(
        program.invoke_builtin_method(&copy_on_write_remove, Some(&copy_on_write_invocation),),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&copy_on_write_size, Some(&copy_on_write_size_invocation)),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&copy_on_write_remove, Some(&copy_on_write_invocation),),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &add,
            Some(&test_invocation(vec![
                DexValue::Reference(std::rc::Rc::from("indexed-list")),
                DexValue::Reference(std::rc::Rc::from("right")),
            ])),
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &list_add_at,
            Some(&test_invocation(vec![
                DexValue::Reference(std::rc::Rc::from("indexed-list")),
                DexValue::Int(0),
                DexValue::Reference(std::rc::Rc::from("left")),
            ])),
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &array_list_add_at,
            Some(&test_invocation(vec![
                DexValue::Reference(std::rc::Rc::from("indexed-list")),
                DexValue::Int(1),
                DexValue::Reference(std::rc::Rc::from("middle")),
            ])),
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&list_size, Some(&indexed_list_invocation)),
        Some(DexExecutionResult::Int(3))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &list_get,
            Some(&test_invocation(vec![
                DexValue::Reference(std::rc::Rc::from("indexed-list")),
                DexValue::Int(0),
            ])),
        ),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("left")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get,
            Some(&test_invocation(vec![
                DexValue::Reference(std::rc::Rc::from("indexed-list")),
                DexValue::Int(1),
            ])),
        ),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("middle")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &array_list_set,
            Some(&test_invocation(vec![
                DexValue::Reference(std::rc::Rc::from("indexed-list")),
                DexValue::Int(1),
                DexValue::Reference(std::rc::Rc::from("new-middle")),
            ])),
        ),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("middle")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get,
            Some(&test_invocation(vec![
                DexValue::Reference(std::rc::Rc::from("indexed-list")),
                DexValue::Int(1),
            ])),
        ),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("new-middle")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&list_size, Some(&list_invocation)),
        Some(DexExecutionResult::Int(2))
    );
    assert_eq!(
        program.invoke_builtin_method(&array_list_size, Some(&list_invocation)),
        Some(DexExecutionResult::Int(2))
    );
    assert_eq!(
        program.invoke_builtin_method(&list_is_empty, Some(&list_invocation)),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&array_list_is_empty, Some(&list_invocation)),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&get, Some(&get_invocation)),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("value")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&list_get, Some(&get_invocation)),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("value")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &collection_constructor,
            Some(&test_invocation(vec![
                DexValue::Reference(std::rc::Rc::from("copied-list")),
                DexValue::Reference(std::rc::Rc::from("list")),
            ])),
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&array_list_size, Some(&copied_list_invocation)),
        Some(DexExecutionResult::Int(2))
    );
    assert_eq!(
        program.invoke_builtin_method(&remove, Some(&get_invocation)),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("value")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&array_list_size, Some(&list_invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert!(matches!(
        program.invoke_builtin_method(&array_list_iterator, Some(&list_invocation)),
        Some(DexExecutionResult::Value(DexValue::Reference(_)))
    ));
    assert_eq!(
        program.invoke_builtin_method(&clear, Some(&list_invocation)),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&array_list_size, Some(&list_invocation)),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&list_add, Some(&invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&list_clear, Some(&list_invocation)),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&array_list_size, Some(&list_invocation)),
        Some(DexExecutionResult::Int(0))
    );
}

#[test]
fn builtin_collections_singleton_list_stores_one_value() {
    let program = DexProgram::new(Vec::new());
    let singleton_list = DexMethodId {
        class_descriptor: "Ljava/util/Collections;".to_owned(),
        name: "singletonList".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/util/List;".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let list_size = DexMethodId {
        class_descriptor: "Ljava/util/List;".to_owned(),
        name: "size".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let list_get = DexMethodId {
        class_descriptor: "Ljava/util/List;".to_owned(),
        name: "get".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LI".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let value = DexValue::Reference(std::rc::Rc::from("work-request"));
    let result =
        program.invoke_builtin_method(&singleton_list, Some(&test_invocation(vec![value.clone()])));
    let Some(DexExecutionResult::Value(list)) = result else {
        panic!("singletonList did not return a list reference");
    };

    assert_eq!(
        program.invoke_builtin_method(&list_size, Some(&test_invocation(vec![list.clone()]))),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &list_get,
            Some(&test_invocation(vec![list, DexValue::Int(0)])),
        ),
        Some(DexExecutionResult::Value(value))
    );
}

#[test]
fn builtin_arrays_as_list_copies_array_entries_into_list_storage() {
    let program = DexProgram::new(Vec::new());
    let as_list = DexMethodId {
        class_descriptor: "Ljava/util/Arrays;".to_owned(),
        name: "asList".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/util/List;".to_owned(),
            parameters: vec!["[Ljava/lang/Object;".to_owned()],
        },
    };
    let list_size = DexMethodId {
        class_descriptor: "Ljava/util/List;".to_owned(),
        name: "size".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let list_get = DexMethodId {
        class_descriptor: "Ljava/util/List;".to_owned(),
        name: "get".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LI".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let right = DexValue::String(std::rc::Rc::from("right"));
    let array = program.allocate_filled_array(
        "[Ljava/lang/Object;".to_owned(),
        vec![DexValue::String(std::rc::Rc::from("left")), right.clone()],
    );
    let Some(DexExecutionResult::Value(list)) =
        program.invoke_builtin_method(&as_list, Some(&test_invocation(vec![array])))
    else {
        panic!("Arrays.asList should return a list object");
    };

    assert_eq!(
        program.invoke_builtin_method(&list_size, Some(&test_invocation(vec![list.clone()]))),
        Some(DexExecutionResult::Int(2))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &list_get,
            Some(&test_invocation(vec![list, DexValue::Int(1)])),
        ),
        Some(DexExecutionResult::Value(right))
    );
}

#[test]
fn builtin_arrays_fill_long_range_updates_array_entries() {
    let program = DexProgram::new(Vec::new());
    let fill = DexMethodId {
        class_descriptor: "Ljava/util/Arrays;".to_owned(),
        name: "fill".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V[IIIJ".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "[J".to_owned(),
                "I".to_owned(),
                "I".to_owned(),
                "J".to_owned(),
            ],
        },
    };
    let array = program.allocate_filled_array(
        "[J".to_owned(),
        vec![
            DexValue::Long(1),
            DexValue::Long(2),
            DexValue::Long(3),
            DexValue::Long(4),
        ],
    );

    let result = program.invoke_builtin_method(
        &fill,
        Some(&test_invocation(vec![
            array.clone(),
            DexValue::Int(1),
            DexValue::Int(3),
            DexValue::Long(9),
            DexValue::Long(9),
        ])),
    );

    assert_eq!(result, Some(DexExecutionResult::Void));
    assert_eq!(
        program
            .runtime
            .borrow()
            .array_entries
            .get(&map_identity_key(&array)),
        Some(&vec![
            DexValue::Long(1),
            DexValue::Long(9),
            DexValue::Long(9),
            DexValue::Long(4),
        ])
    );
}

#[test]
fn builtin_priority_queue_constructor_initializes_managed_collection() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Ljava/util/PriorityQueue;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VIL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned(), "Ljava/util/Comparator;".to_owned()],
        },
    };
    let queue = DexValue::Object(DexObjectRef {
        id: 44,
        class_descriptor: std::rc::Rc::from("Ljava/util/PriorityQueue;"),
    });
    let comparator = DexValue::Reference(std::rc::Rc::from("Ljava/util/Comparator;:test"));

    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![
                queue.clone(),
                DexValue::Int(11),
                comparator
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .list_entries
            .get(&map_identity_key(&queue)),
        Some(&Vec::new())
    );
}

#[test]
fn builtin_priority_queue_size_reads_managed_collection_entries() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/util/PriorityQueue;".to_owned(),
        name: "size".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let queue = DexValue::Object(DexObjectRef {
        id: 44,
        class_descriptor: std::rc::Rc::from("Ljava/util/PriorityQueue;"),
    });
    program.runtime.borrow_mut().list_entries.insert(
        map_identity_key(&queue),
        vec![DexValue::Int(1), DexValue::Int(2)],
    );

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![queue]))),
        Some(DexExecutionResult::Int(2))
    );
}

#[test]
fn builtin_priority_queue_add_stores_non_null_object_value() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/util/PriorityQueue;".to_owned(),
        name: "add".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let queue = DexValue::Object(DexObjectRef {
        id: 44,
        class_descriptor: std::rc::Rc::from("Ljava/util/PriorityQueue;"),
    });
    let value = DexValue::String(std::rc::Rc::from("item"));

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![queue.clone(), value.clone()]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .list_entries
            .get(&map_identity_key(&queue)),
        Some(&vec![value])
    );
}

#[test]
fn builtin_priority_queue_iterator_reads_managed_collection_entries() {
    let program = DexProgram::new(Vec::new());
    let iterator_method = DexMethodId {
        class_descriptor: "Ljava/util/PriorityQueue;".to_owned(),
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
    let queue = DexValue::Object(DexObjectRef {
        id: 44,
        class_descriptor: std::rc::Rc::from("Ljava/util/PriorityQueue;"),
    });
    let first = DexValue::String(std::rc::Rc::from("first"));
    let second = DexValue::String(std::rc::Rc::from("second"));
    program.runtime.borrow_mut().list_entries.insert(
        map_identity_key(&queue),
        vec![first.clone(), second.clone()],
    );

    let Some(DexExecutionResult::Value(iterator)) =
        program.invoke_builtin_method(&iterator_method, Some(&test_invocation(vec![queue])))
    else {
        panic!("PriorityQueue.iterator should return an iterator");
    };
    assert_eq!(
        program.invoke_builtin_method(&has_next, Some(&test_invocation(vec![iterator.clone()]))),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&next, Some(&test_invocation(vec![iterator.clone()]))),
        Some(DexExecutionResult::Value(first))
    );
    assert_eq!(
        program.invoke_builtin_method(&next, Some(&test_invocation(vec![iterator]))),
        Some(DexExecutionResult::Value(second))
    );
}

#[test]
fn builtin_arrays_fill_int_range_updates_array_entries() {
    let program = DexProgram::new(Vec::new());
    let fill = DexMethodId {
        class_descriptor: "Ljava/util/Arrays;".to_owned(),
        name: "fill".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V[IIII".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "[I".to_owned(),
                "I".to_owned(),
                "I".to_owned(),
                "I".to_owned(),
            ],
        },
    };
    let array = program.allocate_filled_array(
        "[I".to_owned(),
        vec![
            DexValue::Int(1),
            DexValue::Int(2),
            DexValue::Int(3),
            DexValue::Int(4),
        ],
    );

    let result = program.invoke_builtin_method(
        &fill,
        Some(&test_invocation(vec![
            array.clone(),
            DexValue::Int(1),
            DexValue::Int(3),
            DexValue::Int(-1),
        ])),
    );

    assert_eq!(result, Some(DexExecutionResult::Void));
    assert_eq!(
        program
            .runtime
            .borrow()
            .array_entries
            .get(&map_identity_key(&array)),
        Some(&vec![
            DexValue::Int(1),
            DexValue::Int(-1),
            DexValue::Int(-1),
            DexValue::Int(4),
        ])
    );
}

#[test]
fn builtin_arrays_sort_int_range_updates_array_entries() {
    let program = DexProgram::new(Vec::new());
    let sort = DexMethodId {
        class_descriptor: "Ljava/util/Arrays;".to_owned(),
        name: "sort".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V[III".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["[I".to_owned(), "I".to_owned(), "I".to_owned()],
        },
    };
    let array = program.allocate_filled_array(
        "[I".to_owned(),
        vec![
            DexValue::Int(9),
            DexValue::Int(4),
            DexValue::Int(7),
            DexValue::Int(1),
            DexValue::Int(6),
        ],
    );

    let result = program.invoke_builtin_method(
        &sort,
        Some(&test_invocation(vec![
            array.clone(),
            DexValue::Int(1),
            DexValue::Int(4),
        ])),
    );

    assert_eq!(result, Some(DexExecutionResult::Void));
    assert_eq!(
        program
            .runtime
            .borrow()
            .array_entries
            .get(&map_identity_key(&array)),
        Some(&vec![
            DexValue::Int(9),
            DexValue::Int(1),
            DexValue::Int(4),
            DexValue::Int(7),
            DexValue::Int(6),
        ])
    );
}

#[test]
fn builtin_arrays_fill_object_range_updates_array_entries() {
    let program = DexProgram::new(Vec::new());
    let fill = DexMethodId {
        class_descriptor: "Ljava/util/Arrays;".to_owned(),
        name: "fill".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V[LL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "[Ljava/lang/Object;".to_owned(),
                "I".to_owned(),
                "I".to_owned(),
                "Ljava/lang/Object;".to_owned(),
            ],
        },
    };
    let array = program.allocate_filled_array(
        "[Ljava/lang/Object;".to_owned(),
        vec![
            DexValue::String(std::rc::Rc::from("keep-left")),
            DexValue::String(std::rc::Rc::from("old-a")),
            DexValue::String(std::rc::Rc::from("old-b")),
            DexValue::String(std::rc::Rc::from("keep-right")),
        ],
    );
    let value = DexValue::Reference(std::rc::Rc::from("fill-object"));

    let result = program.invoke_builtin_method(
        &fill,
        Some(&test_invocation(vec![
            array.clone(),
            DexValue::Int(1),
            DexValue::Int(3),
            value.clone(),
        ])),
    );

    assert_eq!(result, Some(DexExecutionResult::Void));
    assert_eq!(
        program
            .runtime
            .borrow()
            .array_entries
            .get(&map_identity_key(&array)),
        Some(&vec![
            DexValue::String(std::rc::Rc::from("keep-left")),
            value.clone(),
            value,
            DexValue::String(std::rc::Rc::from("keep-right")),
        ])
    );
}

#[test]
fn builtin_arrays_equals_object_compares_array_entries() {
    let program = DexProgram::new(Vec::new());
    let equals = DexMethodId {
        class_descriptor: "Ljava/util/Arrays;".to_owned(),
        name: "equals".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZLL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec![
                "[Ljava/lang/Object;".to_owned(),
                "[Ljava/lang/Object;".to_owned(),
            ],
        },
    };
    let left = program.allocate_filled_array(
        "[Ljava/lang/Object;".to_owned(),
        vec![DexValue::String(std::rc::Rc::from("same"))],
    );
    let equal = program.allocate_filled_array(
        "[Ljava/lang/Object;".to_owned(),
        vec![DexValue::String(std::rc::Rc::from("same"))],
    );
    let different = program.allocate_filled_array(
        "[Ljava/lang/Object;".to_owned(),
        vec![DexValue::String(std::rc::Rc::from("different"))],
    );

    assert_eq!(
        program.invoke_builtin_method(
            &equals,
            Some(&test_invocation(vec![left.clone(), left.clone()]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&equals, Some(&test_invocation(vec![left.clone(), equal]))),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &equals,
            Some(&test_invocation(vec![left.clone(), different]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&equals, Some(&test_invocation(vec![left, DexValue::Null]))),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &equals,
            Some(&test_invocation(vec![DexValue::Null, DexValue::Null]))
        ),
        Some(DexExecutionResult::Int(1))
    );
}

#[test]
fn builtin_arrays_copy_of_object_resizes_array_entries() {
    let program = DexProgram::new(Vec::new());
    let copy_of = DexMethodId {
        class_descriptor: "Ljava/util/Arrays;".to_owned(),
        name: "copyOf".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LLI".to_owned(),
            return_type: "[Ljava/lang/Object;".to_owned(),
            parameters: vec!["[Ljava/lang/Object;".to_owned(), "I".to_owned()],
        },
    };
    let left = program.allocate_object("Lrw;".to_owned());
    let right = program.allocate_object("Lrw;".to_owned());
    let array =
        program.allocate_filled_array("[Lrw;".to_owned(), vec![left.clone(), right.clone()]);

    let Some(DexExecutionResult::Value(grown)) = program.invoke_builtin_method(
        &copy_of,
        Some(&test_invocation(vec![array.clone(), DexValue::Int(3)])),
    ) else {
        panic!("Arrays.copyOf should return grown array");
    };
    let Some(DexExecutionResult::Value(truncated)) = program.invoke_builtin_method(
        &copy_of,
        Some(&test_invocation(vec![array, DexValue::Int(1)])),
    ) else {
        panic!("Arrays.copyOf should return truncated array");
    };

    assert_eq!(value_class_descriptor(&grown), Some("[Lrw;"));
    assert_eq!(value_class_descriptor(&truncated), Some("[Lrw;"));
    assert_eq!(
        program
            .runtime
            .borrow()
            .array_entries
            .get(&map_identity_key(&grown)),
        Some(&vec![left.clone(), right, DexValue::Null])
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .array_entries
            .get(&map_identity_key(&truncated)),
        Some(&vec![left])
    );
}

#[test]
fn builtin_arrays_copy_of_int_resizes_array_entries() {
    let program = DexProgram::new(Vec::new());
    let copy_of = DexMethodId {
        class_descriptor: "Ljava/util/Arrays;".to_owned(),
        name: "copyOf".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LLI".to_owned(),
            return_type: "[I".to_owned(),
            parameters: vec!["[I".to_owned(), "I".to_owned()],
        },
    };
    let array =
        program.allocate_filled_array("[I".to_owned(), vec![DexValue::Int(4), DexValue::Int(7)]);

    let Some(DexExecutionResult::Value(grown)) = program.invoke_builtin_method(
        &copy_of,
        Some(&test_invocation(vec![array.clone(), DexValue::Int(4)])),
    ) else {
        panic!("Arrays.copyOf should return grown int array");
    };
    let Some(DexExecutionResult::Value(truncated)) = program.invoke_builtin_method(
        &copy_of,
        Some(&test_invocation(vec![array, DexValue::Int(1)])),
    ) else {
        panic!("Arrays.copyOf should return truncated int array");
    };

    assert_eq!(
        program
            .runtime
            .borrow()
            .array_entries
            .get(&map_identity_key(&grown)),
        Some(&vec![
            DexValue::Int(4),
            DexValue::Int(7),
            DexValue::Int(0),
            DexValue::Int(0),
        ])
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .array_entries
            .get(&map_identity_key(&truncated)),
        Some(&vec![DexValue::Int(4)])
    );
}

#[test]
fn builtin_arrays_copy_of_long_resizes_array_entries() {
    let program = DexProgram::new(Vec::new());
    let copy_of = DexMethodId {
        class_descriptor: "Ljava/util/Arrays;".to_owned(),
        name: "copyOf".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LLI".to_owned(),
            return_type: "[J".to_owned(),
            parameters: vec!["[J".to_owned(), "I".to_owned()],
        },
    };
    let array =
        program.allocate_filled_array("[J".to_owned(), vec![DexValue::Long(4), DexValue::Long(7)]);

    let Some(DexExecutionResult::Value(grown)) = program.invoke_builtin_method(
        &copy_of,
        Some(&test_invocation(vec![array.clone(), DexValue::Int(4)])),
    ) else {
        panic!("Arrays.copyOf should return grown long array");
    };
    let Some(DexExecutionResult::Value(truncated)) = program.invoke_builtin_method(
        &copy_of,
        Some(&test_invocation(vec![array, DexValue::Int(1)])),
    ) else {
        panic!("Arrays.copyOf should return truncated long array");
    };

    assert_eq!(
        program
            .runtime
            .borrow()
            .array_entries
            .get(&map_identity_key(&grown)),
        Some(&vec![
            DexValue::Long(4),
            DexValue::Long(7),
            DexValue::Long(0),
            DexValue::Long(0),
        ])
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .array_entries
            .get(&map_identity_key(&truncated)),
        Some(&vec![DexValue::Long(4)])
    );
}

#[test]
fn builtin_arrays_sort_object_range_allows_empty_range_without_comparator() {
    let program = DexProgram::new(Vec::new());
    let left = DexValue::Reference(std::rc::Rc::from("left"));
    let right = DexValue::Reference(std::rc::Rc::from("right"));
    let array = program.allocate_filled_array(
        "[Ljava/lang/Object;".to_owned(),
        vec![left.clone(), right.clone()],
    );

    program
        .arrays_sort_object_range_with_comparator(
            &test_invocation(vec![
                array.clone(),
                DexValue::Int(1),
                DexValue::Int(1),
                DexValue::Null,
            ]),
            0,
        )
        .expect("empty sort range should be a valid no-op");

    assert_eq!(program.read_array_value(&array, 0), Some(left));
    assert_eq!(program.read_array_value(&array, 1), Some(right));
    assert!(program
        .arrays_sort_object_range_with_comparator(
            &test_invocation(vec![
                array,
                DexValue::Int(0),
                DexValue::Int(2),
                DexValue::Null,
            ]),
            0,
        )
        .is_err());

    let empty_array = program.allocate_filled_array("[Ljava/lang/Object;".to_owned(), Vec::new());
    program
        .arrays_sort_object_with_comparator(&test_invocation(vec![empty_array, DexValue::Null]), 0)
        .expect("empty whole-array sort should be a valid no-op");

    let nonempty_array = program.allocate_filled_array(
        "[Ljava/lang/Object;".to_owned(),
        vec![
            DexValue::Reference(std::rc::Rc::from("right")),
            DexValue::Reference(std::rc::Rc::from("left")),
        ],
    );
    assert!(program
        .arrays_sort_object_with_comparator(
            &test_invocation(vec![nonempty_array, DexValue::Null]),
            0,
        )
        .is_err());
}

#[test]
fn builtin_array_list_to_array_fills_supplied_array_when_large_enough() {
    let program = DexProgram::new(Vec::new());
    let add = DexMethodId {
        class_descriptor: "Ljava/util/ArrayList;".to_owned(),
        name: "add".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let to_array = DexMethodId {
        class_descriptor: "Ljava/util/ArrayList;".to_owned(),
        name: "toArray".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "[Ljava/lang/Object;".to_owned(),
            parameters: vec!["[Ljava/lang/Object;".to_owned()],
        },
    };
    let list = program.allocate_object("Ljava/util/ArrayList;".to_owned());
    let first = DexValue::Reference(std::rc::Rc::from("first"));
    let second = DexValue::Reference(std::rc::Rc::from("second"));
    let target = program
        .allocate_array("[LA0/a;".to_owned(), 3)
        .expect("target array should allocate");

    program
        .write_array_value(&target, 2, DexValue::Reference(std::rc::Rc::from("old")))
        .expect("array write should succeed");
    assert_eq!(
        program.invoke_builtin_method(
            &add,
            Some(&test_invocation(vec![list.clone(), first.clone()])),
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &add,
            Some(&test_invocation(vec![list.clone(), second.clone()])),
        ),
        Some(DexExecutionResult::Int(1))
    );

    assert_eq!(
        program.invoke_builtin_method(
            &to_array,
            Some(&test_invocation(vec![list, target.clone()])),
        ),
        Some(DexExecutionResult::Value(target.clone()))
    );
    assert_eq!(program.read_array_value(&target, 0), Some(first));
    assert_eq!(program.read_array_value(&target, 1), Some(second));
    assert_eq!(program.read_array_value(&target, 2), Some(DexValue::Null));
}

#[test]
fn builtin_array_list_to_array_allocates_same_descriptor_when_target_is_too_small() {
    let program = DexProgram::new(Vec::new());
    let add = DexMethodId {
        class_descriptor: "Ljava/util/ArrayList;".to_owned(),
        name: "add".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let to_array = DexMethodId {
        class_descriptor: "Ljava/util/ArrayList;".to_owned(),
        name: "toArray".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "[Ljava/lang/Object;".to_owned(),
            parameters: vec!["[Ljava/lang/Object;".to_owned()],
        },
    };
    let list = program.allocate_object("Ljava/util/ArrayList;".to_owned());
    let first = DexValue::Reference(std::rc::Rc::from("first"));
    let second = DexValue::Reference(std::rc::Rc::from("second"));
    let target = program
        .allocate_array("[LA0/a;".to_owned(), 1)
        .expect("target array should allocate");

    assert_eq!(
        program.invoke_builtin_method(
            &add,
            Some(&test_invocation(vec![list.clone(), first.clone()])),
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &add,
            Some(&test_invocation(vec![list.clone(), second.clone()])),
        ),
        Some(DexExecutionResult::Int(1))
    );

    let Some(DexExecutionResult::Value(result)) =
        program.invoke_builtin_method(&to_array, Some(&test_invocation(vec![list, target])))
    else {
        panic!("ArrayList.toArray should return an array");
    };

    assert_ne!(value_class_descriptor(&result), Some("[Ljava/lang/Object;"));
    assert_eq!(value_class_descriptor(&result), Some("[LA0/a;"));
    assert_eq!(program.array_length(&result), Some(2));
    assert_eq!(program.read_array_value(&result, 0), Some(first));
    assert_eq!(program.read_array_value(&result, 1), Some(second));
}

#[test]
fn builtin_abstract_collection_to_array_uses_collection_storage() {
    let program = DexProgram::new(Vec::new());
    let to_array = DexMethodId {
        class_descriptor: "Ljava/util/AbstractCollection;".to_owned(),
        name: "toArray".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "[Ljava/lang/Object;".to_owned(),
            parameters: vec!["[Ljava/lang/Object;".to_owned()],
        },
    };
    let collection = DexValue::Object(DexObjectRef {
        id: 1,
        class_descriptor: std::rc::Rc::from("Ljava/util/AbstractSet;"),
    });
    let first = DexValue::String(std::rc::Rc::from("first"));
    let second = DexValue::String(std::rc::Rc::from("second"));
    let target = program
        .allocate_array("[Ljava/lang/Object;".to_owned(), 0)
        .expect("target array should allocate");

    program.runtime.borrow_mut().list_entries.insert(
        map_identity_key(&collection),
        vec![first.clone(), second.clone()],
    );

    let Some(DexExecutionResult::Value(result)) =
        program.invoke_builtin_method(&to_array, Some(&test_invocation(vec![collection, target])))
    else {
        panic!("AbstractCollection.toArray should return an array");
    };

    assert_eq!(program.array_length(&result), Some(2));
    assert_eq!(program.read_array_value(&result, 0), Some(first));
    assert_eq!(program.read_array_value(&result, 1), Some(second));
}

#[test]
fn builtin_hash_set_is_empty_uses_set_storage() {
    let program = DexProgram::new(Vec::new());
    let add = DexMethodId {
        class_descriptor: "Ljava/util/HashSet;".to_owned(),
        name: "add".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let is_empty = DexMethodId {
        class_descriptor: "Ljava/util/HashSet;".to_owned(),
        name: "isEmpty".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
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
        program.invoke_builtin_method(&is_empty, Some(&test_invocation(vec![set.clone()]))),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &add,
            Some(&test_invocation(vec![
                set.clone(),
                DexValue::String(std::rc::Rc::from("value")),
            ])),
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&is_empty, Some(&test_invocation(vec![set.clone()]))),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&size, Some(&test_invocation(vec![set]))),
        Some(DexExecutionResult::Int(1))
    );
}

#[test]
fn java_array_list_add_all_at_inserts_projected_collection_values() {
    let program = DexProgram::new(Vec::new());
    let add_all = DexMethodId {
        class_descriptor: "Ljava/util/ArrayList;".to_owned(),
        name: "addAll".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZIL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["I".to_owned(), "Ljava/util/Collection;".to_owned()],
        },
    };
    let target = program.allocate_object("Ljava/util/ArrayList;".to_owned());
    let source = program.allocate_object("Ljava/util/ArrayList;".to_owned());
    let target_key = map_identity_key(&target);
    let source_key = map_identity_key(&source);

    program.runtime.borrow_mut().list_entries.insert(
        target_key.clone(),
        vec![
            DexValue::String(std::rc::Rc::from("first")),
            DexValue::String(std::rc::Rc::from("last")),
        ],
    );
    program.runtime.borrow_mut().list_entries.insert(
        source_key,
        vec![
            DexValue::String(std::rc::Rc::from("middle-a")),
            DexValue::String(std::rc::Rc::from("middle-b")),
        ],
    );

    assert_eq!(
        program
            .invoke_java_collection_equality_method(
                &add_all,
                &test_invocation(vec![target.clone(), DexValue::Int(1), source]),
                0,
            )
            .expect("ArrayList.addAll(int, Collection) should run"),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.runtime.borrow().list_entries.get(&target_key),
        Some(&vec![
            DexValue::String(std::rc::Rc::from("first")),
            DexValue::String(std::rc::Rc::from("middle-a")),
            DexValue::String(std::rc::Rc::from("middle-b")),
            DexValue::String(std::rc::Rc::from("last")),
        ])
    );
}

#[test]
fn builtin_array_list_sub_list_clear_removes_parent_range() {
    let program = DexProgram::new(Vec::new());
    let sub_list = DexMethodId {
        class_descriptor: "Ljava/util/ArrayList;".to_owned(),
        name: "subList".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LII".to_owned(),
            return_type: "Ljava/util/List;".to_owned(),
            parameters: vec!["I".to_owned(), "I".to_owned()],
        },
    };
    let get = DexMethodId {
        class_descriptor: "Ljava/util/List;".to_owned(),
        name: "get".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LI".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let size = DexMethodId {
        class_descriptor: "Ljava/util/List;".to_owned(),
        name: "size".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let clear = DexMethodId {
        class_descriptor: "Ljava/util/List;".to_owned(),
        name: "clear".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let list = program.allocate_object("Ljava/util/ArrayList;".to_owned());
    let list_key = map_identity_key(&list);

    program.runtime.borrow_mut().list_entries.insert(
        list_key.clone(),
        vec![
            DexValue::String(std::rc::Rc::from("first")),
            DexValue::String(std::rc::Rc::from("middle")),
            DexValue::String(std::rc::Rc::from("last")),
        ],
    );

    let Some(DexExecutionResult::Value(slice)) = program.invoke_builtin_method(
        &sub_list,
        Some(&test_invocation(vec![
            list,
            DexValue::Int(1),
            DexValue::Int(3),
        ])),
    ) else {
        panic!("ArrayList.subList should return a list");
    };

    assert_eq!(
        program.invoke_builtin_method(&size, Some(&test_invocation(vec![slice.clone()]))),
        Some(DexExecutionResult::Int(2))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get,
            Some(&test_invocation(vec![slice.clone(), DexValue::Int(0)]))
        ),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("middle")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&clear, Some(&test_invocation(vec![slice.clone()]))),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&size, Some(&test_invocation(vec![slice]))),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.runtime.borrow().list_entries.get(&list_key),
        Some(&vec![DexValue::String(std::rc::Rc::from("first"))])
    );
}

#[test]
fn builtin_array_list_remove_object_removes_matching_value() {
    let program = DexProgram::new(Vec::new());
    let add = DexMethodId {
        class_descriptor: "Ljava/util/ArrayList;".to_owned(),
        name: "add".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let remove = DexMethodId {
        class_descriptor: "Ljava/util/ArrayList;".to_owned(),
        name: "remove".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let size = DexMethodId {
        class_descriptor: "Ljava/util/ArrayList;".to_owned(),
        name: "size".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let list = program.allocate_object("Ljava/util/ArrayList;".to_owned());
    let first = DexValue::Reference(std::rc::Rc::from("first"));
    let second = DexValue::Reference(std::rc::Rc::from("second"));

    assert_eq!(
        program.invoke_builtin_method(&add, Some(&test_invocation(vec![list.clone(), first]))),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &add,
            Some(&test_invocation(vec![list.clone(), second.clone()])),
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&remove, Some(&test_invocation(vec![list.clone(), second])),),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&size, Some(&test_invocation(vec![list]))),
        Some(DexExecutionResult::Int(1))
    );
}

#[test]
fn builtin_array_list_index_of_returns_matching_index() {
    let program = DexProgram::new(Vec::new());
    let add = DexMethodId {
        class_descriptor: "Ljava/util/ArrayList;".to_owned(),
        name: "add".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let index_of = DexMethodId {
        class_descriptor: "Ljava/util/ArrayList;".to_owned(),
        name: "indexOf".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "IL".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let list = program.allocate_object("Ljava/util/ArrayList;".to_owned());
    let first = DexValue::Reference(std::rc::Rc::from("first"));
    let second = DexValue::Reference(std::rc::Rc::from("second"));
    let missing = DexValue::Reference(std::rc::Rc::from("missing"));

    assert_eq!(
        program.invoke_builtin_method(
            &add,
            Some(&test_invocation(vec![list.clone(), first.clone()])),
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &add,
            Some(&test_invocation(vec![list.clone(), second.clone()])),
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program
            .invoke_builtin_method(&index_of, Some(&test_invocation(vec![list.clone(), first])),),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &index_of,
            Some(&test_invocation(vec![list.clone(), second])),
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&index_of, Some(&test_invocation(vec![list, missing]))),
        Some(DexExecutionResult::Int(-1))
    );
}

#[test]
fn builtin_collections_empty_list_returns_empty_list() {
    let program = DexProgram::new(Vec::new());
    let empty_list = DexMethodId {
        class_descriptor: "Ljava/util/Collections;".to_owned(),
        name: "emptyList".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/util/List;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let list_size = DexMethodId {
        class_descriptor: "Ljava/util/List;".to_owned(),
        name: "size".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let list_is_empty = DexMethodId {
        class_descriptor: "Ljava/util/List;".to_owned(),
        name: "isEmpty".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let result = program.invoke_builtin_method(&empty_list, None);
    let Some(DexExecutionResult::Value(list)) = result else {
        panic!("emptyList did not return a list reference");
    };
    let invocation = test_invocation(vec![list]);

    assert_eq!(
        program.invoke_builtin_method(&list_size, Some(&invocation)),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&list_is_empty, Some(&invocation)),
        Some(DexExecutionResult::Int(1))
    );
}

#[test]
fn java_array_list_collection_constructor_accepts_collections_empty_list() {
    let program = DexProgram::new(Vec::new());
    let empty_list = DexMethodId {
        class_descriptor: "Ljava/util/Collections;".to_owned(),
        name: "emptyList".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/util/List;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let target = program.allocate_object("Ljava/util/ArrayList;".to_owned());
    let Some(DexExecutionResult::Value(source)) = program.invoke_builtin_method(&empty_list, None)
    else {
        panic!("emptyList did not return a list reference");
    };

    program
        .initialize_array_list_from_collection_iterating(
            &test_invocation(vec![target.clone(), source]),
            0,
        )
        .expect("ArrayList(Collection) should accept Collections.emptyList()");

    assert_eq!(
        program
            .runtime
            .borrow()
            .list_entries
            .get(&map_identity_key(&target)),
        Some(&Vec::<DexValue>::new())
    );
}

#[test]
fn builtin_list_size_declines_object_receivers_without_managed_entries() {
    let program = DexProgram::new(Vec::new());
    let list_size = DexMethodId {
        class_descriptor: "Ljava/util/List;".to_owned(),
        name: "size".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let list_is_empty = DexMethodId {
        class_descriptor: "Ljava/util/List;".to_owned(),
        name: "isEmpty".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let apk_list = DexValue::Object(DexObjectRef {
        id: 99,
        class_descriptor: std::rc::Rc::from("Lexample/GeneratedList;"),
    });
    let apk_invocation = test_invocation(vec![apk_list]);
    let java_list = DexValue::Object(DexObjectRef {
        id: 100,
        class_descriptor: std::rc::Rc::from("Ljava/util/ArrayList;"),
    });
    let java_invocation = test_invocation(vec![java_list]);

    assert_eq!(
        program.invoke_builtin_method(&list_size, Some(&apk_invocation)),
        None
    );
    assert_eq!(
        program.invoke_builtin_method(&list_is_empty, Some(&apk_invocation)),
        None
    );
    assert_eq!(
        program.invoke_builtin_method(&list_size, Some(&java_invocation)),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&list_is_empty, Some(&java_invocation)),
        Some(DexExecutionResult::Int(1))
    );
}

#[test]
fn builtin_list_remove_removes_managed_array_list_value() {
    let program = DexProgram::new(Vec::new());
    let list_remove = DexMethodId {
        class_descriptor: "Ljava/util/List;".to_owned(),
        name: "remove".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LI".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let list_size = DexMethodId {
        class_descriptor: "Ljava/util/List;".to_owned(),
        name: "size".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let list = program.allocate_object("Ljava/util/ArrayList;".to_owned());
    let first = DexValue::Reference(std::rc::Rc::from("first"));
    let second = DexValue::Reference(std::rc::Rc::from("second"));

    program
        .runtime
        .borrow_mut()
        .list_entries
        .insert(map_identity_key(&list), vec![first.clone(), second.clone()]);

    assert_eq!(
        program.invoke_builtin_method(
            &list_remove,
            Some(&test_invocation(vec![list.clone(), DexValue::Int(0)])),
        ),
        Some(DexExecutionResult::Value(first))
    );
    assert_eq!(
        program.invoke_builtin_method(&list_size, Some(&test_invocation(vec![list]))),
        Some(DexExecutionResult::Int(1))
    );
}

#[test]
fn builtin_list_remove_declines_object_receivers_without_managed_entries() {
    let program = DexProgram::new(Vec::new());
    let list_remove = DexMethodId {
        class_descriptor: "Ljava/util/List;".to_owned(),
        name: "remove".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LI".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let apk_list = DexValue::Object(DexObjectRef {
        id: 99,
        class_descriptor: std::rc::Rc::from("Lexample/GeneratedList;"),
    });

    assert_eq!(
        program.invoke_builtin_method(
            &list_remove,
            Some(&test_invocation(vec![apk_list, DexValue::Int(0)])),
        ),
        None
    );
}

#[test]
fn builtin_collections_unmodifiable_list_returns_same_list_reference() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/util/Collections;".to_owned(),
        name: "unmodifiableList".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/util/List;".to_owned(),
            parameters: vec!["Ljava/util/List;".to_owned()],
        },
    };
    let list = DexValue::Reference(std::rc::Rc::from("Ljava/util/List;:test-list"));
    let invocation = test_invocation(vec![list.clone()]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Value(list))
    );
}

#[test]
fn builtin_collections_binary_search_reports_empty_insertion_point() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/util/Collections;".to_owned(),
        name: "binarySearch".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ILL".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec![
                "Ljava/util/List;".to_owned(),
                "Ljava/lang/Object;".to_owned(),
            ],
        },
    };
    let list = program.allocate_object("Ljava/util/ArrayList;".to_owned());
    let preference = program.allocate_object("Landroidx/preference/PreferenceCategory;".to_owned());

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![list, preference])),),
        Some(DexExecutionResult::Int(-1))
    );
}

#[test]
fn builtin_collections_binary_search_uses_preference_order_field() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/util/Collections;".to_owned(),
        name: "binarySearch".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ILL".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec![
                "Ljava/util/List;".to_owned(),
                "Ljava/lang/Object;".to_owned(),
            ],
        },
    };
    let order_field = DexFieldId {
        class_descriptor: "Landroidx/preference/Preference;".to_owned(),
        type_descriptor: "I".to_owned(),
        name: "f".to_owned(),
    };
    let list = program.allocate_object("Ljava/util/ArrayList;".to_owned());
    let first = program.allocate_object("Landroidx/preference/PreferenceCategory;".to_owned());
    let last = program.allocate_object("Landroidx/preference/PreferenceCategory;".to_owned());
    let target = program.allocate_object("Landroidx/preference/PreferenceCategory;".to_owned());

    assert!(program.write_stored_instance_field(&first, &order_field, DexValue::Int(10)));
    assert!(program.write_stored_instance_field(&last, &order_field, DexValue::Int(30)));
    assert!(program.write_stored_instance_field(&target, &order_field, DexValue::Int(20)));
    program
        .runtime
        .borrow_mut()
        .list_entries
        .insert(map_identity_key(&list), vec![first, last]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![list, target])),),
        Some(DexExecutionResult::Int(-2))
    );
}

#[test]
fn builtin_collections_sort_orders_strings_and_preferences() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/util/Collections;".to_owned(),
        name: "sort".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/util/List;".to_owned()],
        },
    };
    let order_field = DexFieldId {
        class_descriptor: "Landroidx/preference/Preference;".to_owned(),
        type_descriptor: "I".to_owned(),
        name: "f".to_owned(),
    };
    let string_list = program.allocate_object("Ljava/util/ArrayList;".to_owned());
    let nullable_comparator_list = program.allocate_object("Ljava/util/ArrayList;".to_owned());
    let preference_list = program.allocate_object("Ljava/util/ArrayList;".to_owned());
    let first = program.allocate_object("Landroidx/preference/PreferenceCategory;".to_owned());
    let last = program.allocate_object("Landroidx/preference/PreferenceCategory;".to_owned());

    assert!(program.write_stored_instance_field(&first, &order_field, DexValue::Int(10)));
    assert!(program.write_stored_instance_field(&last, &order_field, DexValue::Int(30)));
    {
        let mut runtime = program.runtime.borrow_mut();
        runtime.list_entries.insert(
            map_identity_key(&string_list),
            vec![
                DexValue::String(std::rc::Rc::from("beta")),
                DexValue::String(std::rc::Rc::from("alpha")),
            ],
        );
        runtime.list_entries.insert(
            map_identity_key(&nullable_comparator_list),
            vec![
                DexValue::String(std::rc::Rc::from("gamma")),
                DexValue::String(std::rc::Rc::from("alpha")),
            ],
        );
        runtime.list_entries.insert(
            map_identity_key(&preference_list),
            vec![last.clone(), first.clone()],
        );
    }

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![string_list.clone()]))),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![preference_list.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );

    let runtime = program.runtime.borrow();
    assert_eq!(
        runtime.list_entries.get(&map_identity_key(&string_list)),
        Some(&vec![
            DexValue::String(std::rc::Rc::from("alpha")),
            DexValue::String(std::rc::Rc::from("beta")),
        ])
    );
    assert_eq!(
        runtime
            .list_entries
            .get(&map_identity_key(&preference_list)),
        Some(&vec![first, last])
    );
    drop(runtime);

    program
        .collections_sort_list_with_comparator(
            &test_invocation(vec![nullable_comparator_list.clone(), DexValue::Null]),
            0,
        )
        .expect("null comparator should use natural ordering");
    assert_eq!(
        program
            .runtime
            .borrow()
            .list_entries
            .get(&map_identity_key(&nullable_comparator_list)),
        Some(&vec![
            DexValue::String(std::rc::Rc::from("alpha")),
            DexValue::String(std::rc::Rc::from("gamma")),
        ])
    );
    assert!(program
        .collections_sort_list_with_comparator(
            &test_invocation(vec![
                nullable_comparator_list,
                DexValue::Reference(std::rc::Rc::from("Ljava/util/Comparator;:missing-compare")),
            ]),
            0,
        )
        .is_err());
}

#[test]
fn snapshots_androidx_preference_tree_in_order() {
    let program = DexProgram::new(Vec::new());
    let children_field = DexFieldId {
        class_descriptor: "Landroidx/preference/PreferenceGroup;".to_owned(),
        type_descriptor: "Ljava/util/ArrayList;".to_owned(),
        name: "N".to_owned(),
    };
    let title_field = DexFieldId {
        class_descriptor: "Landroidx/preference/Preference;".to_owned(),
        type_descriptor: "Ljava/lang/CharSequence;".to_owned(),
        name: "g".to_owned(),
    };
    let summary_field = DexFieldId {
        class_descriptor: "Landroidx/preference/Preference;".to_owned(),
        type_descriptor: "Ljava/lang/CharSequence;".to_owned(),
        name: "h".to_owned(),
    };
    let key_field = DexFieldId {
        class_descriptor: "Landroidx/preference/Preference;".to_owned(),
        type_descriptor: "Ljava/lang/String;".to_owned(),
        name: "k".to_owned(),
    };
    let root = program.allocate_object("Landroidx/preference/PreferenceScreen;".to_owned());
    let category = program.allocate_object("Landroidx/preference/PreferenceCategory;".to_owned());
    let preference = program.allocate_object("Landroidx/preference/Preference;".to_owned());
    let root_children = program.allocate_object("Ljava/util/ArrayList;".to_owned());
    let category_children = program.allocate_object("Ljava/util/ArrayList;".to_owned());

    assert!(program.write_stored_instance_field(&root, &children_field, root_children.clone(),));
    assert!(program.write_stored_instance_field(
        &category,
        &children_field,
        category_children.clone(),
    ));
    assert!(program.write_stored_instance_field(
        &category,
        &title_field,
        DexValue::String(std::rc::Rc::from("General")),
    ));
    assert!(program.write_stored_instance_field(
        &preference,
        &title_field,
        DexValue::String(std::rc::Rc::from("Language")),
    ));
    assert!(program.write_stored_instance_field(
        &preference,
        &summary_field,
        DexValue::String(std::rc::Rc::from("English")),
    ));
    assert!(program.write_stored_instance_field(
        &preference,
        &key_field,
        DexValue::String(std::rc::Rc::from("com.example.LANGUAGE")),
    ));
    program
        .runtime
        .borrow_mut()
        .list_entries
        .insert(map_identity_key(&root_children), vec![category.clone()]);
    program.runtime.borrow_mut().list_entries.insert(
        map_identity_key(&category_children),
        vec![preference.clone()],
    );

    assert_eq!(
        program.preference_snapshot(),
        vec![
            DexPreferenceSnapshot {
                runtime_key: map_identity_key(&category),
                key: None,
                title: "General".to_owned(),
                summary: None,
                icon_resource_id: None,
                class_descriptor: "Landroidx/preference/PreferenceCategory;".to_owned(),
                kind: DexPreferenceKind::Category,
                depth: 0,
            },
            DexPreferenceSnapshot {
                runtime_key: map_identity_key(&preference),
                key: Some("com.example.LANGUAGE".to_owned()),
                title: "Language".to_owned(),
                summary: Some("English".to_owned()),
                icon_resource_id: None,
                class_descriptor: "Landroidx/preference/Preference;".to_owned(),
                kind: DexPreferenceKind::Plain,
                depth: 1,
            },
        ]
    );
}

#[test]
fn snapshots_two_state_preference_checked_state() {
    let program = DexProgram::new(Vec::new());
    let children_field = DexFieldId {
        class_descriptor: "Landroidx/preference/PreferenceGroup;".to_owned(),
        type_descriptor: "Ljava/util/ArrayList;".to_owned(),
        name: "N".to_owned(),
    };
    let title_field = DexFieldId {
        class_descriptor: "Landroidx/preference/Preference;".to_owned(),
        type_descriptor: "Ljava/lang/CharSequence;".to_owned(),
        name: "g".to_owned(),
    };
    let checked_field = DexFieldId {
        class_descriptor: "Landroidx/preference/TwoStatePreference;".to_owned(),
        type_descriptor: "Z".to_owned(),
        name: "M".to_owned(),
    };
    let icon_field = DexFieldId {
        class_descriptor: "Landroidx/preference/Preference;".to_owned(),
        type_descriptor: "Landroid/graphics/drawable/Drawable;".to_owned(),
        name: "j".to_owned(),
    };
    let root = program.allocate_object("Landroidx/preference/PreferenceScreen;".to_owned());
    let toggle = program.allocate_object("Landroidx/preference/TwoStatePreference;".to_owned());
    let root_children = program.allocate_object("Ljava/util/ArrayList;".to_owned());

    assert!(program.write_stored_instance_field(&root, &children_field, root_children.clone()));
    assert!(program.write_stored_instance_field(
        &toggle,
        &title_field,
        DexValue::String(std::rc::Rc::from("Vibrate")),
    ));
    assert!(program.write_stored_instance_field(&toggle, &checked_field, DexValue::Int(1),));
    assert!(program.write_stored_instance_field(
        &toggle,
        &icon_field,
        DexValue::Reference(std::rc::Rc::from(
            "Landroid/graphics/drawable/Drawable;:resource:2131230888"
        )),
    ));
    program
        .runtime
        .borrow_mut()
        .list_entries
        .insert(map_identity_key(&root_children), vec![toggle.clone()]);

    assert_eq!(
        program.preference_snapshot(),
        vec![DexPreferenceSnapshot {
            runtime_key: map_identity_key(&toggle),
            key: None,
            title: "Vibrate".to_owned(),
            summary: None,
            icon_resource_id: Some(2131230888),
            class_descriptor: "Landroidx/preference/TwoStatePreference;".to_owned(),
            kind: DexPreferenceKind::Switch { checked: true },
            depth: 0,
        }]
    );
}

#[test]
fn snapshots_list_preference_entries_and_values() {
    let program = DexProgram::new(Vec::new());
    let children_field = DexFieldId {
        class_descriptor: "Landroidx/preference/PreferenceGroup;".to_owned(),
        type_descriptor: "Ljava/util/ArrayList;".to_owned(),
        name: "N".to_owned(),
    };
    let title_field = DexFieldId {
        class_descriptor: "Landroidx/preference/Preference;".to_owned(),
        type_descriptor: "Ljava/lang/CharSequence;".to_owned(),
        name: "g".to_owned(),
    };
    let entries_field = DexFieldId {
        class_descriptor: "Landroidx/preference/ListPreference;".to_owned(),
        type_descriptor: "[Ljava/lang/CharSequence;".to_owned(),
        name: "S".to_owned(),
    };
    let entry_values_field = DexFieldId {
        class_descriptor: "Landroidx/preference/ListPreference;".to_owned(),
        type_descriptor: "[Ljava/lang/CharSequence;".to_owned(),
        name: "T".to_owned(),
    };
    let value_field = DexFieldId {
        class_descriptor: "Landroidx/preference/ListPreference;".to_owned(),
        type_descriptor: "Ljava/lang/String;".to_owned(),
        name: "U".to_owned(),
    };
    let root = program.allocate_object("Landroidx/preference/PreferenceScreen;".to_owned());
    let preference = program.allocate_object("Landroidx/preference/ListPreference;".to_owned());
    let root_children = program.allocate_object("Ljava/util/ArrayList;".to_owned());
    let entries = program.allocate_filled_array(
        "[Ljava/lang/CharSequence;".to_owned(),
        vec![
            DexValue::String(std::rc::Rc::from("10")),
            DexValue::String(std::rc::Rc::from("50")),
        ],
    );
    let entry_values = program.allocate_filled_array(
        "[Ljava/lang/CharSequence;".to_owned(),
        vec![
            DexValue::String(std::rc::Rc::from("10")),
            DexValue::String(std::rc::Rc::from("50")),
        ],
    );

    assert!(program.write_stored_instance_field(&root, &children_field, root_children.clone()));
    assert!(program.write_stored_instance_field(
        &preference,
        &title_field,
        DexValue::String(std::rc::Rc::from("Number of decimal places")),
    ));
    assert!(program.write_stored_instance_field(&preference, &entries_field, entries));
    assert!(program.write_stored_instance_field(&preference, &entry_values_field, entry_values,));
    assert!(program.write_stored_instance_field(
        &preference,
        &value_field,
        DexValue::String(std::rc::Rc::from("50")),
    ));
    program
        .runtime
        .borrow_mut()
        .list_entries
        .insert(map_identity_key(&root_children), vec![preference.clone()]);

    assert_eq!(
        program.preference_snapshot(),
        vec![DexPreferenceSnapshot {
            runtime_key: map_identity_key(&preference),
            key: None,
            title: "Number of decimal places".to_owned(),
            summary: None,
            icon_resource_id: None,
            class_descriptor: "Landroidx/preference/ListPreference;".to_owned(),
            kind: DexPreferenceKind::List {
                value: Some("50".to_owned()),
                entries: vec!["10".to_owned(), "50".to_owned()],
                entry_values: vec!["10".to_owned(), "50".to_owned()],
            },
            depth: 0,
        }]
    );
}

// cite: libcore Collections.java#reverse
#[test]
fn collections_reverse_reverses_list_entries_in_place() {
    let program = DexProgram::new(Vec::new());
    let list = program.allocate_object("Ljava/util/ArrayList;".to_owned());
    program.runtime.borrow_mut().list_entries.insert(
        map_identity_key(&list),
        vec![DexValue::Int(1), DexValue::Int(2), DexValue::Int(3)],
    );
    let method = DexMethodId {
        class_descriptor: "Ljava/util/Collections;".to_owned(),
        name: "reverse".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/util/List;".to_owned()],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![list.clone()]))),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .list_entries
            .get(&map_identity_key(&list)),
        Some(&vec![DexValue::Int(3), DexValue::Int(2), DexValue::Int(1)])
    );
}

#[test]
fn libcore_concurrent_linked_queue_constructor_creates_an_empty_queue() {
    // cite: libcore ConcurrentLinkedQueue.java#ConcurrentLinkedQueue
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/ConcurrentLinkedQueue;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
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
    let queue = program.allocate_object("Ljava/util/concurrent/ConcurrentLinkedQueue;".to_owned());

    assert_eq!(
        program.invoke_builtin_method(&constructor, Some(&test_invocation(vec![queue.clone()]))),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&size, Some(&test_invocation(vec![queue.clone()]))),
        Some(DexExecutionResult::Int(0))
    );
    assert!(program.value_matches_type(&queue, "Ljava/util/Queue;"));
    assert!(program.value_matches_type(&queue, "Ljava/util/AbstractCollection;"));
}
