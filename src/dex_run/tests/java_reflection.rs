use super::super::*;
use super::helpers::*;

#[test]
fn builtin_class_to_string_formats_class_kind_and_name() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Class;".to_owned(),
        name: "toString".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: Vec::new(),
        },
    };

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![DexValue::Reference(
                std::rc::Rc::from("Ljava/lang/Class;:Ljava/lang/String;")
            )]))
        ),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("class java.lang.String")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![DexValue::Reference(
                std::rc::Rc::from("Ljava/lang/Class;:Ljava/lang/Runnable;")
            )]))
        ),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("interface java.lang.Runnable")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![DexValue::Reference(
                std::rc::Rc::from("Ljava/lang/Class;:I")
            )]))
        ),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("int")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![DexValue::Reference(
                std::rc::Rc::from("Ljava/lang/Class;:[Ljava/lang/String;")
            )]))
        ),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("class [Ljava.lang.String;")
        )))
    );
}

#[test]
fn builtin_class_get_name_returns_java_name_for_class_reference() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Class;".to_owned(),
        name: "getName".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let invocation = test_invocation(vec![DexValue::Reference(std::rc::Rc::from(
        "Ljava/lang/Class;:Landroidx/work/Worker;",
    ))]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("androidx.work.Worker")
        )))
    );
}

#[test]
fn builtin_class_get_canonical_name_returns_java_name_for_class_reference() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Class;".to_owned(),
        name: "getCanonicalName".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let invocation = test_invocation(vec![DexValue::Reference(std::rc::Rc::from(
        "Ljava/lang/Class;:Landroidx/lifecycle/ViewModel;",
    ))]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("androidx.lifecycle.ViewModel")
        )))
    );
}

#[test]
fn builtin_class_get_simple_name_returns_leaf_name() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Class;".to_owned(),
        name: "getSimpleName".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: Vec::new(),
        },
    };

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![DexValue::Reference(
                std::rc::Rc::from("Ljava/lang/Class;:Landroid/view/View;")
            )]))
        ),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("View")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![DexValue::Reference(
                std::rc::Rc::from("Ljava/lang/Class;:Lcom/example/SettingsActivity$a;")
            )]))
        ),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("a")
        )))
    );
}

// cite: art java_lang_Class.cc#Class_classForName
#[test]
fn art_class_for_name_throws_class_not_found_exception_for_missing_class() {
    let program = DexProgram::new(Vec::new());
    let invocation = test_invocation(vec![DexValue::String(std::rc::Rc::from(
        "androidx.lifecycle.MissingAdapter",
    ))]);

    let DexExecutionResult::Throw(exception) = program
        .class_for_name(&invocation, 0)
        .expect("Class.forName should execute")
    else {
        panic!("missing class did not throw ClassNotFoundException");
    };

    assert!(program.value_matches_type(&exception, "Ljava/lang/ClassNotFoundException;"));
    let detail_message = DexFieldId {
        class_descriptor: "Ljava/lang/Throwable;".to_owned(),
        type_descriptor: "Ljava/lang/String;".to_owned(),
        name: "detailMessage".to_owned(),
    };
    assert_eq!(
        program.read_stored_instance_field(&exception, &detail_message),
        Some(DexValue::String(std::rc::Rc::from(
            "androidx.lifecycle.MissingAdapter"
        )))
    );
}

// cite: art java_lang_Class.cc#Class_classForName
#[test]
fn art_class_for_name_overloads_throw_null_pointer_exception_for_null_name() {
    let program = DexProgram::new(Vec::new());
    let invocations = [
        test_invocation(vec![DexValue::Null]),
        test_invocation(vec![
            DexValue::Null,
            DexValue::Int(0),
            DexValue::Reference(std::rc::Rc::from("Ljava/lang/ClassLoader;.bootstrap")),
        ]),
    ];
    let results = [
        program.class_for_name(&invocations[0], 0),
        program.class_for_name_with_loader(&invocations[1], 0),
    ];

    for result in results {
        let DexExecutionResult::Throw(exception) =
            result.expect("Class.forName null name should execute")
        else {
            panic!("Class.forName null name did not throw NullPointerException");
        };
        assert!(program.value_matches_type(&exception, "Ljava/lang/NullPointerException;"));
    }
}

// cite: art java_lang_Class.cc#Class_classForName
#[test]
fn art_class_for_name_with_loader_returns_loaded_class_reference() {
    let program = DexProgram::new(Vec::new());
    let invocation = test_invocation(vec![
        DexValue::String(std::rc::Rc::from("android.os.Build$VERSION")),
        DexValue::Int(0),
        DexValue::Reference(std::rc::Rc::from("Ljava/lang/ClassLoader;.bootstrap")),
    ]);

    assert_eq!(
        program
            .class_for_name_with_loader(&invocation, 0)
            .expect("Class.forName with loader should execute"),
        DexExecutionResult::Value(DexValue::Reference(std::rc::Rc::from(
            "Ljava/lang/Class;:Landroid/os/Build$VERSION;"
        )))
    );
}

#[test]
// cite: libcore ClassLoader.java#loadClass
fn builtin_class_loader_load_class_returns_loaded_class_reference() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/ClassLoader;".to_owned(),
        name: "loadClass".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/lang/Class;".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned()],
        },
    };
    let invocation = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from("Ljava/lang/ClassLoader;.bootstrap")),
        DexValue::String(std::rc::Rc::from("android.os.Build$VERSION")),
    ]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Ljava/lang/Class;:Landroid/os/Build$VERSION;")
        )))
    );
}

#[test]
// cite: libcore ClassLoader.java#loadClass
fn builtin_class_loader_load_class_throws_for_missing_class() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/ClassLoader;".to_owned(),
        name: "loadClass".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/lang/Class;".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned()],
        },
    };
    let invocation = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from("Ljava/lang/ClassLoader;.bootstrap")),
        DexValue::String(std::rc::Rc::from(
            "androidx.compose.ui.platform.MissingOwner",
        )),
    ]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Throw(DexValue::Object(DexObjectRef {
            id: 1,
            class_descriptor: std::rc::Rc::from("Ljava/lang/ClassNotFoundException;"),
        })))
    );
}

#[test]
// cite: libcore ClassLoader.java#loadClass
fn builtin_class_loader_load_class_throws_for_null_name() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/ClassLoader;".to_owned(),
        name: "loadClass".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/lang/Class;".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned()],
        },
    };
    let invocation = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from("Ljava/lang/ClassLoader;.bootstrap")),
        DexValue::Null,
    ]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Throw(DexValue::Object(DexObjectRef {
            id: 1,
            class_descriptor: std::rc::Rc::from("Ljava/lang/NullPointerException;"),
        })))
    );
}

#[test]
fn builtin_class_get_class_loader_returns_bootstrap_loader() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Class;".to_owned(),
        name: "getClassLoader".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/ClassLoader;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let invocation = test_invocation(vec![DexValue::Reference(std::rc::Rc::from(
        "Ljava/lang/Class;:Landroid/os/Build$VERSION;",
    ))]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Ljava/lang/ClassLoader;.bootstrap")
        )))
    );
}

// cite: art java_lang_Class.cc#Class_classForName
#[test]
fn builtin_reflect_field_get_int_reads_bootstrap_static_field() {
    let program = DexProgram::new(Vec::new());
    let get_field = DexMethodId {
        class_descriptor: "Ljava/lang/Class;".to_owned(),
        name: "getField".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/lang/reflect/Field;".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned()],
        },
    };
    let get_int = DexMethodId {
        class_descriptor: "Ljava/lang/reflect/Field;".to_owned(),
        name: "getInt".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "IL".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let get = DexMethodId {
        class_descriptor: "Ljava/lang/reflect/Field;".to_owned(),
        name: "get".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };

    let DexExecutionResult::Value(class_reference) = program
        .class_for_name(
            &test_invocation(vec![DexValue::String(std::rc::Rc::from(
                "android.os.Build$VERSION",
            ))]),
            0,
        )
        .expect("Class.forName should execute")
    else {
        panic!("forName did not return a class reference");
    };
    let Some(DexExecutionResult::Value(field_reference)) = program.invoke_builtin_method(
        &get_field,
        Some(&test_invocation(vec![
            class_reference,
            DexValue::String(std::rc::Rc::from("SDK_INT")),
        ])),
    ) else {
        panic!("getField did not return a field reference");
    };

    assert_eq!(
        program.invoke_builtin_method(
            &get_int,
            Some(&test_invocation(vec![
                field_reference.clone(),
                DexValue::Null
            ])),
        ),
        Some(DexExecutionResult::Int(BOOTSTRAP_ANDROID_SDK_INT))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get,
            Some(&test_invocation(vec![field_reference, DexValue::Null])),
        ),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from(format!("Ljava/lang/Integer;:{BOOTSTRAP_ANDROID_SDK_INT}"))
        )))
    );
}

// cite: art java_lang_Class.cc#Class_classForName
#[test]
fn builtin_get_declared_field_reads_unsafe_singleton_field() {
    let program = DexProgram::new(Vec::new());
    let get_declared_field = DexMethodId {
        class_descriptor: "Ljava/lang/Class;".to_owned(),
        name: "getDeclaredField".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/lang/reflect/Field;".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned()],
        },
    };
    let get = DexMethodId {
        class_descriptor: "Ljava/lang/reflect/Field;".to_owned(),
        name: "get".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };

    let DexExecutionResult::Value(class_reference) = program
        .class_for_name(
            &test_invocation(vec![DexValue::String(std::rc::Rc::from("sun.misc.Unsafe"))]),
            0,
        )
        .expect("Class.forName should execute")
    else {
        panic!("forName did not return Unsafe class reference");
    };
    let Some(DexExecutionResult::Value(field_reference)) = program.invoke_builtin_method(
        &get_declared_field,
        Some(&test_invocation(vec![
            class_reference,
            DexValue::String(std::rc::Rc::from("theUnsafe")),
        ])),
    ) else {
        panic!("getDeclaredField did not return theUnsafe field reference");
    };

    assert_eq!(
        program.invoke_builtin_method(
            &get,
            Some(&test_invocation(vec![field_reference, DexValue::Null]))
        ),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Lsun/misc/Unsafe;:theUnsafe")
        )))
    );
}

#[test]
fn builtin_reflect_array_new_instance_allocates_array_from_class_reference() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/reflect/Array;".to_owned(),
        name: "newInstance".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LLI".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: vec!["Ljava/lang/Class;".to_owned(), "I".to_owned()],
        },
    };

    let Some(DexExecutionResult::Value(object_array)) = program.invoke_builtin_method(
        &method,
        Some(&test_invocation(vec![
            DexValue::Reference(std::rc::Rc::from("Ljava/lang/Class;:LA0/a;")),
            DexValue::Int(2),
        ])),
    ) else {
        panic!("Array.newInstance should return an object array");
    };
    assert_eq!(
        object_array,
        DexValue::Object(DexObjectRef {
            id: 1,
            class_descriptor: std::rc::Rc::from("[LA0/a;")
        })
    );
    assert_eq!(program.array_length(&object_array), Some(2));
    assert_eq!(
        program
            .runtime
            .borrow()
            .array_entries
            .get(&map_identity_key(&object_array)),
        Some(&vec![DexValue::Null, DexValue::Null])
    );

    let Some(DexExecutionResult::Value(int_array)) = program.invoke_builtin_method(
        &method,
        Some(&test_invocation(vec![
            DexValue::Reference(std::rc::Rc::from("Ljava/lang/Class;:I")),
            DexValue::Int(1),
        ])),
    ) else {
        panic!("Array.newInstance should return a primitive array");
    };
    assert_eq!(
        int_array,
        DexValue::Object(DexObjectRef {
            id: 2,
            class_descriptor: std::rc::Rc::from("[I")
        })
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .array_entries
            .get(&map_identity_key(&int_array)),
        Some(&vec![DexValue::Int(0)])
    );
}

#[test]
fn builtin_object_get_class_returns_receiver_class_reference() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Object;".to_owned(),
        name: "getClass".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/Class;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let invocation = test_invocation(vec![DexValue::Object(DexObjectRef {
        id: 1,
        class_descriptor: std::rc::Rc::from("Lcom/example/Foo;"),
    })]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Ljava/lang/Class;:Lcom/example/Foo;")
        )))
    );
}

#[test]
fn builtin_object_get_class_reads_descriptor_from_reference_receiver() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Object;".to_owned(),
        name: "getClass".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/Class;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let invocation = test_invocation(vec![DexValue::Reference(std::rc::Rc::from(
        "Ljava/lang/Thread;:main",
    ))]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Ljava/lang/Class;:Ljava/lang/Thread;")
        )))
    );
}

#[test]
fn builtin_object_get_class_supports_virtual_dispatch_descriptors() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Lcom/example/Foo;".to_owned(),
        name: "getClass".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/Class;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let invocation = test_invocation(vec![DexValue::Object(DexObjectRef {
        id: 1,
        class_descriptor: std::rc::Rc::from("Lcom/example/Foo;"),
    })]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Ljava/lang/Class;:Lcom/example/Foo;")
        )))
    );
}

#[test]
fn builtin_object_get_class_returns_null_for_null_receiver_until_exceptions_exist() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Object;".to_owned(),
        name: "getClass".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/Class;".to_owned(),
            parameters: Vec::new(),
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![DexValue::Null]))),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
}

#[test]
fn builtin_object_hash_code_returns_stable_identity_hash() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Object;".to_owned(),
        name: "hashCode".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let object_invocation = test_invocation(vec![DexValue::Object(DexObjectRef {
        id: 7,
        class_descriptor: std::rc::Rc::from("Lcom/example/Foo;"),
    })]);
    let reference_invocation = test_invocation(vec![DexValue::Reference(std::rc::Rc::from(
        "Ljava/lang/Thread;:main",
    ))]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&object_invocation)),
        Some(DexExecutionResult::Int(stable_hash_i32("object:7")))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&reference_invocation)),
        Some(DexExecutionResult::Int(stable_hash_i32(
            "reference:Ljava/lang/Thread;:main"
        )))
    );
}

#[test]
fn builtin_objects_require_non_null_returns_non_null_object() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/util/Objects;".to_owned(),
        name: "requireNonNull".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LLL".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: vec![
                "Ljava/lang/Object;".to_owned(),
                "Ljava/lang/String;".to_owned(),
            ],
        },
    };
    let value = DexValue::Object(DexObjectRef {
        id: 7,
        class_descriptor: std::rc::Rc::from("Ljava/io/StringWriter;"),
    });
    let invocation = test_invocation(vec![
        value.clone(),
        DexValue::String(std::rc::Rc::from("out == null")),
    ]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Value(value))
    );
}

#[test]
fn builtin_class_get_package_returns_package_reference() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Class;".to_owned(),
        name: "getPackage".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/Package;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let invocation = test_invocation(vec![DexValue::Reference(std::rc::Rc::from(
        "Ljava/lang/Class;:Landroidx/lifecycle/ViewModel;",
    ))]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Ljava/lang/Package;:androidx.lifecycle")
        )))
    );
}

#[test]
fn builtin_package_get_name_returns_package_name() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Package;".to_owned(),
        name: "getName".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let invocation = test_invocation(vec![DexValue::Reference(std::rc::Rc::from(
        "Ljava/lang/Package;:androidx.lifecycle",
    ))]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("androidx.lifecycle")
        )))
    );
}

#[test]
fn builtin_class_get_declared_fields_returns_dex_fields_and_reads_values() {
    let dex_file = crate::dex::parse_dex_file("classes.dex", minimal_field_class_dex())
        .expect("DEX should parse");
    let program = DexProgram::new(vec![dex_file]);
    let get_declared_fields = DexMethodId {
        class_descriptor: "Ljava/lang/Class;".to_owned(),
        name: "getDeclaredFields".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "[Ljava/lang/reflect/Field;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get_name = DexMethodId {
        class_descriptor: "Ljava/lang/reflect/Field;".to_owned(),
        name: "getName".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get_type = DexMethodId {
        class_descriptor: "Ljava/lang/reflect/Field;".to_owned(),
        name: "getType".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/Class;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get_modifiers = DexMethodId {
        class_descriptor: "Ljava/lang/reflect/Field;".to_owned(),
        name: "getModifiers".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get = DexMethodId {
        class_descriptor: "Ljava/lang/reflect/Field;".to_owned(),
        name: "get".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let class_reference =
        DexValue::Reference(std::rc::Rc::from("Ljava/lang/Class;:Lcom/example/Test;"));

    let Some(DexExecutionResult::Value(field_array)) = program.invoke_builtin_method(
        &get_declared_fields,
        Some(&test_invocation(vec![class_reference])),
    ) else {
        panic!("getDeclaredFields did not return an array");
    };
    let fields = program
        .runtime
        .borrow()
        .array_entries
        .get(&map_identity_key(&field_array))
        .cloned()
        .expect("field array entries should exist");
    let static_field = fields[0].clone();
    let instance_field = fields[1].clone();
    let static_field_id = DexFieldId {
        class_descriptor: "Lcom/example/Test;".to_owned(),
        type_descriptor: "I".to_owned(),
        name: "staticValue".to_owned(),
    };
    let instance_field_id = DexFieldId {
        class_descriptor: "Lcom/example/Test;".to_owned(),
        type_descriptor: "Ljava/lang/String;".to_owned(),
        name: "value".to_owned(),
    };
    let object = DexValue::Object(DexObjectRef {
        id: 17,
        class_descriptor: std::rc::Rc::from("Lcom/example/Test;"),
    });

    program.write_stored_static_field(&static_field_id, DexValue::Int(12));
    assert!(program.write_stored_instance_field(
        &object,
        &instance_field_id,
        DexValue::String(std::rc::Rc::from("stored"))
    ));

    assert_eq!(fields.len(), 2);
    assert_eq!(
        program.invoke_builtin_method(
            &get_name,
            Some(&test_invocation(vec![static_field.clone()]))
        ),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("staticValue")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_type,
            Some(&test_invocation(vec![instance_field.clone()]))
        ),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Ljava/lang/Class;:Ljava/lang/String;")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_modifiers,
            Some(&test_invocation(vec![instance_field.clone()]))
        ),
        Some(DexExecutionResult::Int(0x0002))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get,
            Some(&test_invocation(vec![static_field, DexValue::Null]))
        ),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Ljava/lang/Integer;:12")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&get, Some(&test_invocation(vec![instance_field, object]))),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("stored")
        )))
    );
}

#[test]
fn builtin_class_get_declared_constructor_returns_constructor_reference() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Class;".to_owned(),
        name: "getDeclaredConstructor".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/lang/reflect/Constructor;".to_owned(),
            parameters: vec!["[Ljava/lang/Class;".to_owned()],
        },
    };
    let invocation = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from(
            "Ljava/lang/Class;:Landroidx/lifecycle/Foo;",
        )),
        DexValue::Reference(std::rc::Rc::from("[Ljava/lang/Class;:params")),
    ]);

    assert_eq!(
            program.invoke_builtin_method(&method, Some(&invocation)),
            Some(DexExecutionResult::Value(DexValue::Reference(
                std::rc::Rc::from("Ljava/lang/reflect/Constructor;:Landroidx/lifecycle/Foo;:reference:[Ljava/lang/Class;:params")
            )))
        );
}

#[test]
fn builtin_class_get_constructor_returns_constructor_reference() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Class;".to_owned(),
        name: "getConstructor".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/lang/reflect/Constructor;".to_owned(),
            parameters: vec!["[Ljava/lang/Class;".to_owned()],
        },
    };
    let invocation = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from(
            "Ljava/lang/Class;:Landroidx/preference/PreferenceScreen;",
        )),
        DexValue::Reference(std::rc::Rc::from("[Ljava/lang/Class;:params")),
    ]);

    assert_eq!(
            program.invoke_builtin_method(&method, Some(&invocation)),
            Some(DexExecutionResult::Value(DexValue::Reference(
                std::rc::Rc::from("Ljava/lang/reflect/Constructor;:Landroidx/preference/PreferenceScreen;:reference:[Ljava/lang/Class;:params")
            )))
        );
}

#[test]
fn builtin_class_get_declared_constructor_returns_null_for_missing_class_probe() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Class;".to_owned(),
        name: "getDeclaredConstructor".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/lang/reflect/Constructor;".to_owned(),
            parameters: vec!["[Ljava/lang/Class;".to_owned()],
        },
    };
    let invocation = test_invocation(vec![
        DexValue::Null,
        DexValue::Reference(std::rc::Rc::from("[Ljava/lang/Class;:params")),
    ]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
}

#[test]
fn reflect_constructor_reference_decodes_parameter_descriptors() {
    let program = DexProgram::new(Vec::new());
    let parameters = program.allocate_filled_array(
        "[Ljava/lang/Class;".to_owned(),
        vec![
            DexValue::Reference(std::rc::Rc::from("Ljava/lang/Class;:Ljava/lang/String;")),
            DexValue::Reference(std::rc::Rc::from("Ljava/lang/Class;:I")),
        ],
    );
    let parameter_key = map_identity_key(&parameters);
    let constructor = DexValue::Reference(std::rc::Rc::from(format!(
        "Ljava/lang/reflect/Constructor;:Lcom/example/Test;:{parameter_key}"
    )));

    assert_eq!(
        reflect_constructor_reference(&constructor),
        Some(("Lcom/example/Test;", parameter_key.as_str()))
    );
    assert_eq!(
        program.reflect_constructor_parameter_descriptors(&parameter_key),
        Some(vec!["Ljava/lang/String;".to_owned(), "I".to_owned()])
    );
    assert_eq!(
        program.reflect_constructor_parameter_descriptors("int:0"),
        Some(Vec::new())
    );
    assert_eq!(
        proto_shorty("V", &["Ljava/lang/String;".to_owned(), "I".to_owned()]),
        Some("VLI".to_owned())
    );
}

#[test]
fn reflected_no_arg_constructor_allocates_existing_class_without_dex_constructor() {
    let dex_file = crate::dex::parse_dex_file("classes.dex", minimal_field_class_dex())
        .expect("DEX should parse");
    let program = DexProgram::new(vec![dex_file]);
    let invocation = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from(
            "Ljava/lang/reflect/Constructor;:Lcom/example/Test;:int:0",
        )),
        DexValue::Int(0),
    ]);

    assert_eq!(
        program
            .reflect_constructor_new_instance(&invocation, 0)
            .expect("no-arg reflected construction should allocate"),
        DexValue::Object(DexObjectRef {
            id: 1,
            class_descriptor: std::rc::Rc::from("Lcom/example/Test;")
        })
    );
}

#[test]
fn builtin_class_get_method_returns_method_reference() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Class;".to_owned(),
        name: "getMethod".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LLL".to_owned(),
            return_type: "Ljava/lang/reflect/Method;".to_owned(),
            parameters: vec![
                "Ljava/lang/String;".to_owned(),
                "[Ljava/lang/Class;".to_owned(),
            ],
        },
    };
    let invocation = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from("Ljava/lang/Class;:Landroid/view/View;")),
        DexValue::String(std::rc::Rc::from("setOnClickListener")),
        DexValue::Reference(std::rc::Rc::from("[Ljava/lang/Class;:params")),
    ]);

    assert_eq!(
            program.invoke_builtin_method(&method, Some(&invocation)),
            Some(DexExecutionResult::Value(DexValue::Reference(
                std::rc::Rc::from("Ljava/lang/reflect/Method;:Landroid/view/View;:setOnClickListener:reference:[Ljava/lang/Class;:params")
            )))
        );
}

#[test]
fn builtin_class_get_declared_method_returns_method_reference() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Class;".to_owned(),
        name: "getDeclaredMethod".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LLL".to_owned(),
            return_type: "Ljava/lang/reflect/Method;".to_owned(),
            parameters: vec![
                "Ljava/lang/String;".to_owned(),
                "[Ljava/lang/Class;".to_owned(),
            ],
        },
    };
    let invocation = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from("Ljava/lang/Class;:Landroid/os/Handler;")),
        DexValue::String(std::rc::Rc::from("createAsync")),
        DexValue::Reference(std::rc::Rc::from("[Ljava/lang/Class;:params")),
    ]);

    assert_eq!(
            program.invoke_builtin_method(&method, Some(&invocation)),
            Some(DexExecutionResult::Value(DexValue::Reference(
                std::rc::Rc::from("Ljava/lang/reflect/Method;:Landroid/os/Handler;:createAsync:reference:[Ljava/lang/Class;:params")
            )))
        );
}

#[test]
fn builtin_reflect_method_invoke_returns_null() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/reflect/Method;".to_owned(),
        name: "invoke".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LLL".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: vec![
                "Ljava/lang/Object;".to_owned(),
                "[Ljava/lang/Object;".to_owned(),
            ],
        },
    };
    let invocation = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from(
            "Ljava/lang/reflect/Method;:Landroid/view/View;:setOptionalFitsSystemWindows:null"
                .to_owned(),
        )),
        DexValue::Reference(std::rc::Rc::from(
            "Lcom/android/internal/policy/DecorView;.decor",
        )),
        DexValue::Null,
    ]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
}

#[test]
fn builtin_reflect_method_invoke_accepts_zero_null_slots() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/reflect/Method;".to_owned(),
        name: "invoke".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LLL".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: vec![
                "Ljava/lang/Object;".to_owned(),
                "[Ljava/lang/Object;".to_owned(),
            ],
        },
    };
    let invocation = test_invocation(vec![
        DexValue::Reference(
            std::rc::Rc::from("Ljava/lang/reflect/Method;:Landroidx/compose/ui/platform/AndroidCompositionLocals_androidKt;:getLocalSavedStateRegistryOwner:int:0"
                .to_owned()),
        ),
        DexValue::Int(0),
        DexValue::Int(0),
    ]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
}

#[test]
fn builtin_reflect_method_invoke_dispatches_handler_create_async() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/reflect/Method;".to_owned(),
        name: "invoke".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LLL".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: vec![
                "Ljava/lang/Object;".to_owned(),
                "[Ljava/lang/Object;".to_owned(),
            ],
        },
    };
    let looper = DexValue::Reference(std::rc::Rc::from("Landroid/os/Looper;:main"));
    let argument_array =
        program.allocate_filled_array("[Ljava/lang/Object;".to_owned(), vec![looper.clone()]);
    let invocation = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from(
            "Ljava/lang/reflect/Method;:Landroid/os/Handler;:createAsync:params",
        )),
        DexValue::Null,
        argument_array,
    ]);

    let Some(DexExecutionResult::Value(handler)) =
        program.invoke_builtin_method(&method, Some(&invocation))
    else {
        panic!("Method.invoke should return a handler");
    };

    assert!(matches!(
        &handler,
        DexValue::Object(DexObjectRef {
            class_descriptor,
            ..
        }) if class_descriptor.as_ref() == "Landroid/os/Handler;"
    ));
    assert_eq!(
        program
            .runtime
            .borrow()
            .handler_loopers
            .get(&map_identity_key(&handler)),
        Some(&looper)
    );
}

#[test]
fn builtin_reflect_method_invoke_boxes_class_is_record_boolean() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/reflect/Method;".to_owned(),
        name: "invoke".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LLL".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: vec![
                "Ljava/lang/Object;".to_owned(),
                "[Ljava/lang/Object;".to_owned(),
            ],
        },
    };
    let invocation = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from(
            "Ljava/lang/reflect/Method;:Ljava/lang/Class;:isRecord:int:0",
        )),
        DexValue::Reference(std::rc::Rc::from("Ljava/lang/Class;:Ljava/util/ArrayList;")),
        DexValue::Int(0),
    ]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Ljava/lang/Boolean;:false")
        )))
    );
}

#[test]
fn builtin_reflect_method_get_return_type_uses_bootstrap_signature() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/reflect/Method;".to_owned(),
        name: "getReturnType".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/Class;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let invocation = test_invocation(vec![DexValue::Reference(std::rc::Rc::from(
        "Ljava/lang/reflect/Method;:Ljava/lang/Class;:getRecordComponents:int:0",
    ))]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Ljava/lang/Class;:[Ljava/lang/reflect/RecordComponent;")
        )))
    );
}

#[test]
fn builtin_class_get_declared_methods_returns_empty_method_array() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Class;".to_owned(),
        name: "getDeclaredMethods".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "[Ljava/lang/reflect/Method;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let invocation = test_invocation(vec![DexValue::Reference(std::rc::Rc::from(
        "Ljava/lang/Class;:Landroidx/lifecycle/Foo;",
    ))]);
    let Some(DexExecutionResult::Value(array)) =
        program.invoke_builtin_method(&method, Some(&invocation))
    else {
        panic!("getDeclaredMethods should return an array");
    };

    assert_eq!(program.array_length(&array), Some(0));
}

#[test]
fn builtin_class_name_shape_predicates_return_false() {
    let program = DexProgram::new(Vec::new());
    for name in ["isAnonymousClass", "isLocalClass"] {
        let method = DexMethodId {
            class_descriptor: "Ljava/lang/Class;".to_owned(),
            name: name.to_owned(),
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
}

#[test]
fn builtin_class_is_member_class_reads_descriptor_shape() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Class;".to_owned(),
        name: "isMemberClass".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![DexValue::Reference(
                std::rc::Rc::from("Ljava/lang/Class;:Lcom/example/SettingsActivity;")
            )]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![DexValue::Reference(
                std::rc::Rc::from("Ljava/lang/Class;:Lcom/example/SettingsActivity$a;")
            )]))
        ),
        Some(DexExecutionResult::Int(1))
    );
}

#[test]
fn builtin_class_is_array_reads_class_descriptor() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Class;".to_owned(),
        name: "isArray".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let class_invocation = test_invocation(vec![DexValue::Reference(std::rc::Rc::from(
        "Ljava/lang/Class;:Lcom/example/Test;",
    ))]);
    let array_invocation = test_invocation(vec![DexValue::Reference(std::rc::Rc::from(
        "Ljava/lang/Class;:[Ljava/lang/String;",
    ))]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&class_invocation)),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&array_invocation)),
        Some(DexExecutionResult::Int(1))
    );
}

#[test]
fn builtin_class_is_primitive_reads_class_descriptor() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Class;".to_owned(),
        name: "isPrimitive".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let class_invocation = test_invocation(vec![DexValue::Reference(std::rc::Rc::from(
        "Ljava/lang/Class;:Lcom/example/Test;",
    ))]);
    let int_invocation = test_invocation(vec![DexValue::Reference(std::rc::Rc::from(
        "Ljava/lang/Class;:I",
    ))]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&class_invocation)),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&int_invocation)),
        Some(DexExecutionResult::Int(1))
    );
}

#[test]
fn builtin_class_get_component_type_returns_array_component_or_null() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Class;".to_owned(),
        name: "getComponentType".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/Class;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let object_array = test_invocation(vec![DexValue::Reference(std::rc::Rc::from(
        "Ljava/lang/Class;:[Ljava/lang/String;",
    ))]);
    let primitive_array = test_invocation(vec![DexValue::Reference(std::rc::Rc::from(
        "Ljava/lang/Class;:[I",
    ))]);
    let nested_array = test_invocation(vec![DexValue::Reference(std::rc::Rc::from(
        "Ljava/lang/Class;:[[I",
    ))]);
    let non_array = test_invocation(vec![DexValue::Reference(std::rc::Rc::from(
        "Ljava/lang/Class;:Ljava/lang/String;",
    ))]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&object_array)),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Ljava/lang/Class;:Ljava/lang/String;")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&primitive_array)),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Ljava/lang/Class;:I")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&nested_array)),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Ljava/lang/Class;:[I")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&non_array)),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
}

#[test]
fn builtin_class_get_modifiers_reads_bootstrap_and_dex_flags() {
    let dex_file = crate::dex::parse_dex_file("classes.dex", minimal_interface_dex())
        .expect("DEX should parse");
    let program = DexProgram::new(vec![dex_file]);
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Class;".to_owned(),
        name: "getModifiers".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let dex_class = test_invocation(vec![DexValue::Reference(std::rc::Rc::from(
        "Ljava/lang/Class;:Lcom/example/Test;",
    ))]);
    let bootstrap_class = test_invocation(vec![DexValue::Reference(std::rc::Rc::from(
        "Ljava/lang/Class;:Ljava/util/ArrayList;",
    ))]);
    let bootstrap_interface = test_invocation(vec![DexValue::Reference(std::rc::Rc::from(
        "Ljava/lang/Class;:Ljava/util/List;",
    ))]);
    let nested_class = test_invocation(vec![DexValue::Reference(std::rc::Rc::from(
        "Ljava/lang/Class;:Lcom/example/SettingsActivity$a;",
    ))]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&dex_class)),
        Some(DexExecutionResult::Int(
            (JAVA_ACC_PUBLIC | JAVA_ACC_FINAL) as i32
        ))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&bootstrap_class)),
        Some(DexExecutionResult::Int(JAVA_ACC_PUBLIC as i32))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&bootstrap_interface)),
        Some(DexExecutionResult::Int(
            (JAVA_ACC_PUBLIC | JAVA_ACC_INTERFACE | JAVA_ACC_ABSTRACT) as i32
        ))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&nested_class)),
        Some(DexExecutionResult::Int(
            (JAVA_ACC_PUBLIC | DEX_ACC_STATIC) as i32
        ))
    );
}

#[test]
fn builtin_modifier_predicates_check_standard_bits() {
    let program = DexProgram::new(Vec::new());

    for (name, flag) in [
        ("isAbstract", JAVA_ACC_ABSTRACT),
        ("isStatic", DEX_ACC_STATIC),
        ("isTransient", JAVA_ACC_TRANSIENT),
        ("isFinal", JAVA_ACC_FINAL),
        ("isPrivate", JAVA_ACC_PRIVATE),
        ("isProtected", JAVA_ACC_PROTECTED),
        ("isPublic", JAVA_ACC_PUBLIC),
    ] {
        let method = DexMethodId {
            class_descriptor: "Ljava/lang/reflect/Modifier;".to_owned(),
            name: name.to_owned(),
            proto: crate::dex::DexProto {
                shorty: "ZI".to_owned(),
                return_type: "Z".to_owned(),
                parameters: vec!["I".to_owned()],
            },
        };
        let flagged_invocation = test_invocation(vec![DexValue::Int(flag as i32)]);
        let unflagged_invocation = test_invocation(vec![DexValue::Int(0)]);

        assert_eq!(
            program.invoke_builtin_method(&method, Some(&flagged_invocation)),
            Some(DexExecutionResult::Int(1))
        );
        assert_eq!(
            program.invoke_builtin_method(&method, Some(&unflagged_invocation)),
            Some(DexExecutionResult::Int(0))
        );
    }
}

#[test]
fn builtin_class_is_interface_reads_dex_and_bootstrap_flags() {
    let dex_file = crate::dex::parse_dex_file("classes.dex", minimal_interface_dex())
        .expect("DEX should parse");
    let program = DexProgram::new(vec![dex_file]);
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Class;".to_owned(),
        name: "isInterface".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let dex_class = test_invocation(vec![DexValue::Reference(std::rc::Rc::from(
        "Ljava/lang/Class;:Lcom/example/Test;",
    ))]);
    let dex_interface = test_invocation(vec![DexValue::Reference(std::rc::Rc::from(
        "Ljava/lang/Class;:Lcom/example/Marker;",
    ))]);
    let bootstrap_interface = test_invocation(vec![DexValue::Reference(std::rc::Rc::from(
        "Ljava/lang/Class;:Ljava/util/List;",
    ))]);
    let array_class = test_invocation(vec![DexValue::Reference(std::rc::Rc::from(
        "Ljava/lang/Class;:[Ljava/lang/String;",
    ))]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&dex_class)),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&dex_interface)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&bootstrap_interface)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&array_class)),
        Some(DexExecutionResult::Int(0))
    );
}

#[test]
fn builtin_class_get_annotation_returns_null_for_absent_annotation() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Class;".to_owned(),
        name: "getAnnotation".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/lang/annotation/Annotation;".to_owned(),
            parameters: vec!["Ljava/lang/Class;".to_owned()],
        },
    };
    let invocation = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from("Ljava/lang/Class;:Ljava/util/ArrayList;")),
        DexValue::Reference(std::rc::Rc::from("Ljava/lang/Class;:Ld1/a;")),
    ]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
}

#[test]
fn builtin_class_get_interfaces_returns_parsed_interface_classes() {
    let dex_file = crate::dex::parse_dex_file("classes.dex", minimal_interface_dex())
        .expect("DEX should parse");
    let program = DexProgram::new(vec![dex_file]);
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Class;".to_owned(),
        name: "getInterfaces".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "[Ljava/lang/Class;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let invocation = test_invocation(vec![DexValue::Reference(std::rc::Rc::from(
        "Ljava/lang/Class;:Lcom/example/Test;",
    ))]);
    let Some(DexExecutionResult::Value(array)) =
        program.invoke_builtin_method(&method, Some(&invocation))
    else {
        panic!("getInterfaces should return an array");
    };

    assert_eq!(program.array_length(&array), Some(1));
    assert_eq!(
        program.read_array_value(&array, 0),
        Some(DexValue::Reference(std::rc::Rc::from(
            "Ljava/lang/Class;:Lcom/example/Marker;"
        )))
    );
}

#[test]
fn builtin_class_get_generic_interfaces_returns_bootstrap_types() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Class;".to_owned(),
        name: "getGenericInterfaces".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "[Ljava/lang/reflect/Type;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let invocation = test_invocation(vec![DexValue::Reference(std::rc::Rc::from(
        "Ljava/lang/Class;:Ljava/util/ArrayList;",
    ))]);
    let Some(DexExecutionResult::Value(array)) =
        program.invoke_builtin_method(&method, Some(&invocation))
    else {
        panic!("getGenericInterfaces should return an array");
    };

    assert_eq!(program.array_length(&array), Some(4));
    assert_eq!(
        program.read_array_value(&array, 0),
        Some(DexValue::Reference(std::rc::Rc::from(
            "Ljava/lang/Class;:Ljava/util/List;"
        )))
    );
    assert_eq!(
        program.read_array_value(&array, 1),
        Some(DexValue::Reference(std::rc::Rc::from(
            "Ljava/lang/Class;:Ljava/util/RandomAccess;"
        )))
    );
    assert_eq!(
        program.read_array_value(&array, 2),
        Some(DexValue::Reference(std::rc::Rc::from(
            "Ljava/lang/Class;:Ljava/lang/Cloneable;"
        )))
    );
    assert_eq!(
        program.read_array_value(&array, 3),
        Some(DexValue::Reference(std::rc::Rc::from(
            "Ljava/lang/Class;:Ljava/io/Serializable;"
        )))
    );
}

#[test]
fn builtin_class_get_superclass_returns_null_for_object() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Class;".to_owned(),
        name: "getSuperclass".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/Class;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let invocation = test_invocation(vec![DexValue::Reference(std::rc::Rc::from(
        "Ljava/lang/Class;:Ljava/lang/Object;",
    ))]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
}

#[test]
fn builtin_class_get_superclass_uses_bootstrap_hierarchy() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Class;".to_owned(),
        name: "getSuperclass".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/Class;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let invocation = test_invocation(vec![DexValue::Reference(std::rc::Rc::from(
        "Ljava/lang/Class;:Ljava/util/ArrayList;",
    ))]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Ljava/lang/Class;:Ljava/util/AbstractList;")
        )))
    );
}

#[test]
fn builtin_class_get_generic_superclass_uses_bootstrap_hierarchy() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Class;".to_owned(),
        name: "getGenericSuperclass".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/reflect/Type;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let invocation = test_invocation(vec![DexValue::Reference(std::rc::Rc::from(
        "Ljava/lang/Class;:Ljava/util/ArrayList;",
    ))]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Ljava/lang/Class;:Ljava/util/AbstractList;")
        )))
    );
}

#[test]
fn builtin_class_is_assignable_from_handles_exact_object_and_null() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Class;".to_owned(),
        name: "isAssignableFrom".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/Class;".to_owned()],
        },
    };
    let exact = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from(
            "Ljava/lang/Class;:Landroidx/lifecycle/Foo;",
        )),
        DexValue::Reference(std::rc::Rc::from(
            "Ljava/lang/Class;:Landroidx/lifecycle/Foo;",
        )),
    ]);
    let object = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from("Ljava/lang/Class;:Ljava/lang/Object;")),
        DexValue::Reference(std::rc::Rc::from(
            "Ljava/lang/Class;:Landroidx/lifecycle/Foo;",
        )),
    ]);
    let null_candidate = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from(
            "Ljava/lang/Class;:Landroidx/lifecycle/Foo;",
        )),
        DexValue::Null,
    ]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&exact)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&object)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&null_candidate)),
        Some(DexExecutionResult::Int(0))
    );
}

#[test]
fn builtin_class_is_assignable_from_uses_bootstrap_collection_hierarchy() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Class;".to_owned(),
        name: "isAssignableFrom".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/Class;".to_owned()],
        },
    };
    let collection = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from(
            "Ljava/lang/Class;:Ljava/util/Collection;",
        )),
        DexValue::Reference(std::rc::Rc::from("Ljava/lang/Class;:Ljava/util/ArrayList;")),
    ]);
    let list = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from("Ljava/lang/Class;:Ljava/util/List;")),
        DexValue::Reference(std::rc::Rc::from("Ljava/lang/Class;:Ljava/util/ArrayList;")),
    ]);
    let iterable = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from("Ljava/lang/Class;:Ljava/lang/Iterable;")),
        DexValue::Reference(std::rc::Rc::from("Ljava/lang/Class;:Ljava/util/ArrayList;")),
    ]);
    let map = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from("Ljava/lang/Class;:Ljava/util/Map;")),
        DexValue::Reference(std::rc::Rc::from("Ljava/lang/Class;:Ljava/util/ArrayList;")),
    ]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&collection)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&list)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&iterable)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&map)),
        Some(DexExecutionResult::Int(0))
    );
}

#[test]
fn builtin_class_is_assignable_from_uses_bootstrap_view_hierarchy() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Class;".to_owned(),
        name: "isAssignableFrom".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/Class;".to_owned()],
        },
    };
    let frame_layout_is_group = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from(
            "Ljava/lang/Class;:Landroid/view/ViewGroup;",
        )),
        DexValue::Reference(std::rc::Rc::from(
            "Ljava/lang/Class;:Landroid/widget/FrameLayout;",
        )),
    ]);
    let linear_layout_is_view = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from("Ljava/lang/Class;:Landroid/view/View;")),
        DexValue::Reference(std::rc::Rc::from(
            "Ljava/lang/Class;:Landroid/widget/LinearLayout;",
        )),
    ]);
    let button_is_text_view = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from(
            "Ljava/lang/Class;:Landroid/widget/TextView;",
        )),
        DexValue::Reference(std::rc::Rc::from(
            "Ljava/lang/Class;:Landroid/widget/Button;",
        )),
    ]);
    let text_view_is_group = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from(
            "Ljava/lang/Class;:Landroid/view/ViewGroup;",
        )),
        DexValue::Reference(std::rc::Rc::from(
            "Ljava/lang/Class;:Landroid/widget/TextView;",
        )),
    ]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&frame_layout_is_group)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&linear_layout_is_view)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&button_is_text_view)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&text_view_is_group)),
        Some(DexExecutionResult::Int(0))
    );
}

#[test]
fn builtin_class_is_instance_uses_runtime_value_type() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Class;".to_owned(),
        name: "isInstance".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let exact = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from(
            "Ljava/lang/Class;:Landroidx/lifecycle/Foo;",
        )),
        DexValue::Object(DexObjectRef {
            id: 1,
            class_descriptor: std::rc::Rc::from("Landroidx/lifecycle/Foo;"),
        }),
    ]);
    let object = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from("Ljava/lang/Class;:Ljava/lang/Object;")),
        DexValue::Object(DexObjectRef {
            id: 1,
            class_descriptor: std::rc::Rc::from("Landroidx/lifecycle/Foo;"),
        }),
    ]);
    let null_value = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from(
            "Ljava/lang/Class;:Landroidx/lifecycle/Foo;",
        )),
        DexValue::Null,
    ]);
    let serializable_float = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from(
            "Ljava/lang/Class;:Ljava/io/Serializable;",
        )),
        DexValue::Reference(std::rc::Rc::from(format!(
            "Ljava/lang/Float;:{:08x}",
            1.5_f32.to_bits()
        ))),
    ]);
    let serializable_array_list = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from(
            "Ljava/lang/Class;:Ljava/io/Serializable;",
        )),
        program.allocate_object("Ljava/util/ArrayList;".to_owned()),
    ]);
    let parcelable_bundle = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from(
            "Ljava/lang/Class;:Landroid/os/Parcelable;",
        )),
        program.allocate_object("Landroid/os/Bundle;".to_owned()),
    ]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&exact)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&object)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&null_value)),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&serializable_float)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&serializable_array_list)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&parcelable_bundle)),
        Some(DexExecutionResult::Int(1))
    );
}

// cite: libcore Class.java#cast, isInstance, isAssignableFrom
#[test]
fn libcore_class_cast_returns_null_and_assignment_compatible_values() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Class;".to_owned(),
        name: "cast".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let string_array = program
        .allocate_array("[Ljava/lang/String;".to_owned(), 1)
        .expect("array should allocate");
    let exact_array = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from("Ljava/lang/Class;:[Ljava/lang/String;")),
        string_array.clone(),
    ]);
    let object_array = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from("Ljava/lang/Class;:[Ljava/lang/Object;")),
        string_array.clone(),
    ]);
    let object = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from("Ljava/lang/Class;:Ljava/lang/Object;")),
        string_array.clone(),
    ]);
    let list = program.allocate_object("Ljava/util/ArrayList;".to_owned());
    let interface = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from("Ljava/lang/Class;:Ljava/util/List;")),
        list.clone(),
    ]);
    let reference_array = DexValue::Reference(std::rc::Rc::from("[I:styleable"));
    let exact_reference_array = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from("Ljava/lang/Class;:[I")),
        reference_array.clone(),
    ]);
    let reference_array_as_object = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from("Ljava/lang/Class;:Ljava/lang/Object;")),
        reference_array.clone(),
    ]);
    let null_value = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from("Ljava/lang/Class;:[Ljava/lang/String;")),
        DexValue::Null,
    ]);
    let encoded_null = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from("Ljava/lang/Class;:[Ljava/lang/String;")),
        DexValue::Int(0),
    ]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&exact_array)),
        Some(DexExecutionResult::Value(string_array.clone()))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&object_array)),
        Some(DexExecutionResult::Value(string_array.clone()))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&object)),
        Some(DexExecutionResult::Value(string_array))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&interface)),
        Some(DexExecutionResult::Value(list))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&exact_reference_array)),
        Some(DexExecutionResult::Value(reference_array.clone()))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&reference_array_as_object)),
        Some(DexExecutionResult::Value(reference_array))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&null_value)),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&encoded_null)),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
}

// cite: libcore Class.java#cast, cannotCastMsg, getName
// cite: libcore ClassCastException.java#ClassCastException
#[test]
fn libcore_class_cast_throws_catchable_class_cast_exception_with_message() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Class;".to_owned(),
        name: "cast".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let mismatches = [
        (
            test_invocation(vec![
                DexValue::Reference(std::rc::Rc::from("Ljava/lang/Class;:Ljava/lang/String;")),
                program.allocate_object("Ljava/util/ArrayList;".to_owned()),
            ]),
            "Cannot cast java.util.ArrayList to java.lang.String",
        ),
        (
            test_invocation(vec![
                DexValue::Reference(std::rc::Rc::from("Ljava/lang/Class;:[Ljava/lang/Object;")),
                DexValue::Reference(std::rc::Rc::from("[I:styleable")),
            ]),
            "Cannot cast [I to [Ljava.lang.Object;",
        ),
    ];
    let detail_message = DexFieldId {
        class_descriptor: "Ljava/lang/Throwable;".to_owned(),
        type_descriptor: "Ljava/lang/String;".to_owned(),
        name: "detailMessage".to_owned(),
    };

    for (mismatch, expected_message) in mismatches {
        let result = program.invoke_builtin_method(&method, Some(&mismatch));
        let Some(DexExecutionResult::Throw(exception)) = result else {
            panic!("Class.cast mismatch should throw a guest exception: {result:?}");
        };

        assert!(program.value_matches_type(&exception, "Ljava/lang/ClassCastException;"));
        assert!(program.value_matches_type(&exception, "Ljava/lang/RuntimeException;"));
        assert!(program.value_matches_type(&exception, "Ljava/lang/Throwable;"));
        assert_eq!(
            program.read_stored_instance_field(&exception, &detail_message),
            Some(DexValue::String(std::rc::Rc::from(expected_message)))
        );
    }
}

struct ClassCastBuiltinThrowHandler<'a> {
    program: &'a DexProgram,
}

impl DexInvocationHandler for ClassCastBuiltinThrowHandler<'_> {
    fn invoke(&mut self, _invocation: DexInvocation) -> Result<DexInvocationResult> {
        let method = DexMethodId {
            class_descriptor: "Ljava/lang/Class;".to_owned(),
            name: "cast".to_owned(),
            proto: crate::dex::DexProto {
                shorty: "LL".to_owned(),
                return_type: "Ljava/lang/Object;".to_owned(),
                parameters: vec!["Ljava/lang/Object;".to_owned()],
            },
        };
        let invocation = test_invocation(vec![
            DexValue::Reference(std::rc::Rc::from("Ljava/lang/Class;:Ljava/lang/String;")),
            self.program
                .allocate_object("Ljava/util/ArrayList;".to_owned()),
        ]);
        let result = self
            .program
            .invoke_builtin_method(&method, Some(&invocation))
            .ok_or_else(|| WinedroidError::new("Class.cast builtin did not handle invocation"))?;

        Ok(invocation_result_from_execution(result))
    }

    fn exception_matches(&mut self, exception: &DexValue, type_descriptor: &str) -> Result<bool> {
        Ok(self.program.value_matches_type(exception, type_descriptor))
    }
}

// cite: libcore Class.java#cast, cannotCastMsg
// cite: libcore ClassCastException.java#ClassCastException
// cite: art interpreter_switch_impl-inl.h#PossiblyHandlePendingExceptionOnInvoke, MOVE_EXCEPTION
// cite: art interpreter_common.cc#MoveToExceptionHandler
#[test]
fn libcore_class_cast_failure_enters_dex_runtime_exception_handler() {
    let program = DexProgram::new(Vec::new());
    let code = DexCodeItem {
        registers_size: 1,
        ins_size: 0,
        outs_size: 0,
        tries_size: 1,
        debug_info_offset: 0,
        instructions: vec![0x0071, 0x0000, 0x0000, 0x000d, 0x0011],
        try_blocks: vec![crate::dex::DexTryBlock {
            start_address: 0,
            instruction_count: 3,
            handlers: vec![crate::dex::DexExceptionHandler {
                type_descriptor: Some("Ljava/lang/RuntimeException;".to_owned()),
                address: 3,
            }],
        }],
    };
    let mut handler = ClassCastBuiltinThrowHandler { program: &program };

    let result = crate::dex_interpreter::interpret_code_with_initial_arguments_in_context(
        &code,
        None,
        &mut handler,
        0,
    )
    .expect("Class.cast failure should remain a guest exception");
    let DexExecutionResult::Value(exception) = result else {
        panic!("RuntimeException handler should return Class.cast failure: {result}");
    };

    assert!(program.value_matches_type(&exception, "Ljava/lang/ClassCastException;"));
}

#[test]
fn builtin_accessible_object_accessibility_methods_are_bootstrapped() {
    let program = DexProgram::new(Vec::new());
    let is_accessible = DexMethodId {
        class_descriptor: "Ljava/lang/reflect/AccessibleObject;".to_owned(),
        name: "isAccessible".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set_accessible = DexMethodId {
        class_descriptor: "Ljava/lang/reflect/AccessibleObject;".to_owned(),
        name: "setAccessible".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VZ".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Z".to_owned()],
        },
    };
    let get_annotations = DexMethodId {
        class_descriptor: "Ljava/lang/reflect/AccessibleObject;".to_owned(),
        name: "getAnnotations".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "[Ljava/lang/annotation/Annotation;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let object = DexValue::Reference(std::rc::Rc::from("Ljava/lang/reflect/Constructor;:example"));
    let is_accessible_invocation = test_invocation(vec![object.clone()]);
    let set_accessible_invocation = test_invocation(vec![object.clone(), DexValue::Int(1)]);

    assert_eq!(
        program.invoke_builtin_method(&is_accessible, Some(&is_accessible_invocation)),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&set_accessible, Some(&set_accessible_invocation)),
        Some(DexExecutionResult::Void)
    );
    let Some(DexExecutionResult::Value(annotations)) =
        program.invoke_builtin_method(&get_annotations, Some(&test_invocation(vec![object])))
    else {
        panic!("AccessibleObject.getAnnotations should return an array");
    };
    assert_eq!(
        program
            .runtime
            .borrow()
            .array_entries
            .get(&map_identity_key(&annotations))
            .map(Vec::len),
        Some(0)
    );
}

#[test]
fn builtin_time_unit_to_millis_converts_reached_units() {
    let program = DexProgram::new(Vec::new());
    let to_millis = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/TimeUnit;".to_owned(),
        name: "toMillis".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "JJ".to_owned(),
            return_type: "J".to_owned(),
            parameters: vec!["J".to_owned()],
        },
    };
    let to_nanos = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/TimeUnit;".to_owned(),
        name: "toNanos".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "JJ".to_owned(),
            return_type: "J".to_owned(),
            parameters: vec!["J".to_owned()],
        },
    };
    let invocation = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from("Ljava/util/concurrent/TimeUnit;.SECONDS")),
        DexValue::Long(3),
        DexValue::Long(3),
    ]);

    assert_eq!(
        program.invoke_builtin_method(&to_millis, Some(&invocation)),
        Some(DexExecutionResult::Value(DexValue::Long(3_000)))
    );
    assert_eq!(
        program.invoke_builtin_method(&to_nanos, Some(&invocation)),
        Some(DexExecutionResult::Value(DexValue::Long(3_000_000_000)))
    );
}

#[test]
fn builtin_time_unit_convert_converts_between_units() {
    let program = DexProgram::new(Vec::new());
    let convert = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/TimeUnit;".to_owned(),
        name: "convert".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "JJL".to_owned(),
            return_type: "J".to_owned(),
            parameters: vec!["J".to_owned(), "Ljava/util/concurrent/TimeUnit;".to_owned()],
        },
    };
    let millis_from_seconds = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from(
            "Ljava/util/concurrent/TimeUnit;.MILLISECONDS",
        )),
        DexValue::Long(2),
        DexValue::Long(2),
        DexValue::Reference(std::rc::Rc::from("Ljava/util/concurrent/TimeUnit;.SECONDS")),
    ]);
    let seconds_from_millis = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from("Ljava/util/concurrent/TimeUnit;.SECONDS")),
        DexValue::Long(1_500),
        DexValue::Long(1_500),
        DexValue::Reference(std::rc::Rc::from(
            "Ljava/util/concurrent/TimeUnit;.MILLISECONDS",
        )),
    ]);

    assert_eq!(
        program.invoke_builtin_method(&convert, Some(&millis_from_seconds)),
        Some(DexExecutionResult::Value(DexValue::Long(2_000)))
    );
    assert_eq!(
        program.invoke_builtin_method(&convert, Some(&seconds_from_millis)),
        Some(DexExecutionResult::Value(DexValue::Long(1)))
    );
}

// cite: libcore Class.java#getAnnotation
#[test]
fn class_get_annotation_returns_runtime_annotation_and_element_value() {
    let file = crate::dex::parse_dex_file("classes.dex", minimal_annotated_class_dex())
        .expect("DEX should parse");
    let program = DexProgram::new(vec![file]);
    let get_annotation = DexMethodId {
        class_descriptor: "Ljava/lang/Class;".to_owned(),
        name: "getAnnotation".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/lang/annotation/Annotation;".to_owned(),
            parameters: vec!["Ljava/lang/Class;".to_owned()],
        },
    };

    let result = program.invoke_builtin_method(
        &get_annotation,
        Some(&test_invocation(vec![
            DexValue::Reference(std::rc::Rc::from("Ljava/lang/Class;:Lcom/example/Test;")),
            DexValue::Reference(std::rc::Rc::from("Ljava/lang/Class;:Lcom/example/Name;")),
        ])),
    );
    let Some(DexExecutionResult::Value(annotation)) = result else {
        panic!("getAnnotation should return a value, got {result:?}");
    };
    assert!(matches!(annotation, DexValue::Object(_)));

    let value_method = DexMethodId {
        class_descriptor: "Lcom/example/Name;".to_owned(),
        name: "value".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: Vec::new(),
        },
    };
    assert_eq!(
        program.annotation_element_value(&value_method, &test_invocation(vec![annotation])),
        Some(DexValue::String(std::rc::Rc::from("hello")))
    );
}

// cite: libcore Class.java#getAnnotation
#[test]
fn class_get_annotation_returns_null_when_annotation_absent() {
    let file = crate::dex::parse_dex_file("classes.dex", minimal_annotated_class_dex())
        .expect("DEX should parse");
    let program = DexProgram::new(vec![file]);
    let get_annotation = DexMethodId {
        class_descriptor: "Ljava/lang/Class;".to_owned(),
        name: "getAnnotation".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/lang/annotation/Annotation;".to_owned(),
            parameters: vec!["Ljava/lang/Class;".to_owned()],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(
            &get_annotation,
            Some(&test_invocation(vec![
                DexValue::Reference(std::rc::Rc::from("Ljava/lang/Class;:Lcom/example/Test;")),
                DexValue::Reference(std::rc::Rc::from("Ljava/lang/Class;:Lcom/example/Missing;")),
            ])),
        ),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
}

// cite: libcore Enum.java#Enum
#[test]
fn enum_descriptor_matches_serializable_and_comparable() {
    let program = DexProgram::new(Vec::new());

    assert!(program.descriptor_matches_type("Ljava/lang/Enum;", "Ljava/io/Serializable;"));
    assert!(program.descriptor_matches_type("Ljava/lang/Enum;", "Ljava/lang/Comparable;"));
}

#[test]
fn art_class_get_declared_field_on_looper_returns_declared_fields_and_throws_for_others() {
    // cite: art java_lang_Class.cc#Class_getDeclaredField, GetDeclaredField
    // cite: frameworks-base Looper.java#mQueue, mThread
    let program = DexProgram::new(Vec::new());
    let get_declared_field = DexMethodId {
        class_descriptor: "Ljava/lang/Class;".to_owned(),
        name: "getDeclaredField".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/lang/reflect/Field;".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned()],
        },
    };
    let looper_class =
        DexValue::Reference(std::rc::Rc::from("Ljava/lang/Class;:Landroid/os/Looper;"));
    let declared = |field_name: &str| {
        program.invoke_builtin_method(
            &get_declared_field,
            Some(&test_invocation(vec![
                looper_class.clone(),
                DexValue::String(std::rc::Rc::from(field_name)),
            ])),
        )
    };

    assert_eq!(
        declared("mQueue"),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from(
                "Ljava/lang/reflect/Field;:Landroid/os/Looper;:mQueue:Landroid/os/MessageQueue;:16"
            )
        )))
    );
    let Some(DexExecutionResult::Throw(exception)) = declared("mBase") else {
        panic!("Looper declares no mBase and should throw NoSuchFieldException");
    };
    assert!(program.value_matches_type(&exception, "Ljava/lang/NoSuchFieldException;"));
}

#[test]
fn art_class_get_declared_field_follows_own_fields_and_throws_no_such_field() {
    // cite: art java_lang_Class.cc#Class_getDeclaredField, GetDeclaredField
    // cite: art common_throws.cc#ThrowNoSuchFieldException
    let program = DexProgram::new(Vec::new());
    let get_declared_field = DexMethodId {
        class_descriptor: "Ljava/lang/Class;".to_owned(),
        name: "getDeclaredField".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/lang/reflect/Field;".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned()],
        },
    };
    let message_field = DexFieldId {
        class_descriptor: "Ljava/lang/Throwable;".to_owned(),
        type_descriptor: "Ljava/lang/String;".to_owned(),
        name: "detailMessage".to_owned(),
    };
    let class_of = |name: &str| {
        let DexExecutionResult::Value(class_reference) = program
            .class_for_name(
                &test_invocation(vec![DexValue::String(std::rc::Rc::from(name))]),
                0,
            )
            .expect("Class.forName should execute")
        else {
            panic!("forName did not return a class reference");
        };
        class_reference
    };
    let declared = |class_name: &str, field_name: DexValue| {
        program.invoke_builtin_method(
            &get_declared_field,
            Some(&test_invocation(vec![class_of(class_name), field_name])),
        )
    };
    let name = |text: &str| DexValue::String(std::rc::Rc::from(text));

    assert_eq!(
        declared("android.content.ContextWrapper", name("mBase")),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from(
                "Ljava/lang/reflect/Field;:Landroid/content/ContextWrapper;:mBase:Landroid/content/Context;:0"
            )
        )))
    );

    let Some(DexExecutionResult::Throw(exception)) =
        declared("android.app.Application", name("mBase"))
    else {
        panic!("superclass field should throw NoSuchFieldException");
    };
    assert!(program.value_matches_type(&exception, "Ljava/lang/NoSuchFieldException;"));
    assert_eq!(
        program.read_stored_instance_field(&exception, &message_field),
        Some(name("No field mBase in class Landroid/app/Application;"))
    );

    let Some(DexExecutionResult::Throw(exception)) =
        declared("android.app.Application", DexValue::Null)
    else {
        panic!("null name should throw NullPointerException");
    };
    assert!(program.value_matches_type(&exception, "Ljava/lang/NullPointerException;"));
    assert_eq!(
        program.read_stored_instance_field(&exception, &message_field),
        Some(name("name == null"))
    );

    let Some(DexExecutionResult::Throw(exception)) =
        declared("android.app.ContextImpl", name("mBase"))
    else {
        panic!("ContextImpl has no mBase field and should throw NoSuchFieldException");
    };
    assert!(program.value_matches_type(&exception, "Ljava/lang/NoSuchFieldException;"));

    // cite: frameworks-base Context.java#ACCOUNT_SERVICE
    assert_eq!(
        declared("android.content.Context", name("ACCOUNT_SERVICE")),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from(
                "Ljava/lang/reflect/Field;:Landroid/content/Context;:ACCOUNT_SERVICE:Ljava/lang/String;:25"
            )
        )))
    );
    let Some(DexExecutionResult::Throw(exception)) =
        declared("android.content.Context", name("mBase"))
    else {
        panic!("Context declares no mBase field and should throw NoSuchFieldException");
    };
    assert!(program.value_matches_type(&exception, "Ljava/lang/NoSuchFieldException;"));
    assert_eq!(
        program.read_stored_instance_field(&exception, &message_field),
        Some(name("No field mBase in class Landroid/content/Context;"))
    );

    // cite: libcore Object.java#Object
    let Some(DexExecutionResult::Throw(exception)) = declared("java.lang.Object", name("mBase"))
    else {
        panic!("Object declares no discoverable field and should throw NoSuchFieldException");
    };
    assert!(program.value_matches_type(&exception, "Ljava/lang/NoSuchFieldException;"));
}

#[test]
fn libcore_no_such_field_exception_constructs_with_message_under_reflective_operation_exception() {
    // cite: libcore NoSuchFieldException.java#NoSuchFieldException
    let program = DexProgram::new(Vec::new());
    let exception = program.allocate_object("Ljava/lang/NoSuchFieldException;".to_owned());
    let init = |parameters: &[&str]| {
        builtin_method(
            "Ljava/lang/NoSuchFieldException;",
            "<init>",
            "V",
            parameters,
        )
    };
    let message = DexValue::String(std::rc::Rc::from("mBase"));

    assert_eq!(
        program.invoke_builtin_method(&init(&[]), Some(&test_invocation(vec![exception.clone()]))),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &init(&["Ljava/lang/String;"]),
            Some(&test_invocation(vec![exception.clone(), message.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(
            &exception,
            &DexFieldId {
                class_descriptor: "Ljava/lang/Throwable;".to_owned(),
                type_descriptor: "Ljava/lang/String;".to_owned(),
                name: "detailMessage".to_owned(),
            }
        ),
        Some(message)
    );
    assert!(program.value_matches_type(&exception, "Ljava/lang/ReflectiveOperationException;"));
    assert!(program.value_matches_type(&exception, "Ljava/lang/Exception;"));
}

#[test]
fn art_class_get_declared_field_on_application_info_returns_primary_cpu_abi() {
    // cite: art java_lang_Class.cc#Class_getDeclaredField, GetDeclaredField
    // cite: frameworks-base ApplicationInfo.java#primaryCpuAbi
    let program = DexProgram::new(Vec::new());
    let get_declared_field = builtin_method(
        "Ljava/lang/Class;",
        "getDeclaredField",
        "Ljava/lang/reflect/Field;",
        &["Ljava/lang/String;"],
    );
    let class = DexValue::Reference(std::rc::Rc::from(
        "Ljava/lang/Class;:Landroid/content/pm/ApplicationInfo;",
    ));
    let declared = |field_name: &str| {
        program.invoke_builtin_method(
            &get_declared_field,
            Some(&test_invocation(vec![
                class.clone(),
                DexValue::String(std::rc::Rc::from(field_name)),
            ])),
        )
    };

    assert_eq!(
        declared("primaryCpuAbi"),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from(
                "Ljava/lang/reflect/Field;:Landroid/content/pm/ApplicationInfo;:primaryCpuAbi:Ljava/lang/String;:1"
            )
        )))
    );
    let Some(DexExecutionResult::Throw(exception)) = declared("notAField") else {
        panic!("ApplicationInfo declares no notAField and should throw NoSuchFieldException");
    };
    assert!(program.value_matches_type(&exception, "Ljava/lang/NoSuchFieldException;"));
}

#[test]
fn frameworks_application_info_primary_cpu_abi_follows_installed_native_abi_directory() {
    // cite: frameworks-base ApplicationInfo.java#primaryCpuAbi
    let app_dir =
        std::env::temp_dir().join(format!("winedroid-primary-cpu-abi-{}", std::process::id()));
    std::fs::remove_dir_all(&app_dir).ok();
    std::fs::create_dir_all(&app_dir).expect("app dir");
    let mut program = DexProgram::new(Vec::new());
    program.apk_path = Some(app_dir.join("source.apk"));
    program.package_name = Some("com.example".to_owned());
    let info = DexValue::Reference(std::rc::Rc::from(
        "Landroid/content/pm/ApplicationInfo;.bootstrap",
    ));
    let field = DexFieldId {
        class_descriptor: "Landroid/content/pm/ApplicationInfo;".to_owned(),
        type_descriptor: "Ljava/lang/String;".to_owned(),
        name: "primaryCpuAbi".to_owned(),
    };

    assert_eq!(program.read_bootstrap_instance_field(&info, &field), None);

    std::fs::create_dir_all(app_dir.join("native/arm64-v8a")).expect("native dir");

    assert_eq!(
        program.read_bootstrap_instance_field(&info, &field),
        Some(DexValue::String(std::rc::Rc::from("arm64-v8a")))
    );
    std::fs::remove_dir_all(app_dir).ok();
}

#[test]
fn libcore_class_as_subclass_returns_self_or_throws_class_cast() {
    // cite: libcore Class.java#asSubclass
    let program = DexProgram::new(Vec::new());
    let as_subclass = builtin_method(
        "Ljava/lang/Class;",
        "asSubclass",
        "Ljava/lang/Class;",
        &["Ljava/lang/Class;"],
    );
    let integer = DexValue::Reference(std::rc::Rc::from("Ljava/lang/Class;:Ljava/lang/Integer;"));
    let string = DexValue::Reference(std::rc::Rc::from("Ljava/lang/Class;:Ljava/lang/String;"));
    let number = DexValue::Reference(std::rc::Rc::from("Ljava/lang/Class;:Ljava/lang/Number;"));

    assert_eq!(
        program.invoke_builtin_method(
            &as_subclass,
            Some(&test_invocation(vec![integer.clone(), number.clone()]))
        ),
        Some(DexExecutionResult::Value(integer))
    );
    let result =
        program.invoke_builtin_method(&as_subclass, Some(&test_invocation(vec![string, number])));
    assert!(program.value_matches_type(&thrown_exception(result), "Ljava/lang/ClassCastException;"));
}

// cite: art java_lang_reflect_Method.cc#Method_getDefaultValue
#[test]
fn builtin_reflect_method_get_default_value_is_null_outside_annotations() {
    let dex_file = crate::dex::parse_dex_file("classes.dex", minimal_interface_dex())
        .expect("DEX should parse");
    let program = DexProgram::new(vec![dex_file]);
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/reflect/Method;".to_owned(),
        name: "getDefaultValue".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let reflected = |class: &str| {
        test_invocation(vec![DexValue::Reference(std::rc::Rc::from(format!(
            "Ljava/lang/reflect/Method;:{class}:run:null"
        )))])
    };

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&reflected("Lcom/example/Test;"))),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&reflected("Lcom/example/Missing;"))),
        None
    );
}
