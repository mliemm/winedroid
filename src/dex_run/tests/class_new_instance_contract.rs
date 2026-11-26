use super::super::*;

fn class_new_instance_contract_program() -> DexProgram {
    let dex_file = crate::dex::parse_dex_file(
        "class_new_instance_contract.dex",
        include_bytes!("fixtures/class_new_instance_contract.dex").to_vec(),
    )
    .expect("Class.newInstance contract DEX should parse");
    DexProgram::new(vec![dex_file])
}

fn execute_class_new_instance_contract_method(name: &str) -> DexExecutionResult {
    let program = class_new_instance_contract_program();
    let method = DexMethodId {
        class_descriptor: "Lwinedroid/tests/ClassNewInstanceContract;".to_owned(),
        name: name.to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let resolved = program
        .resolve_exact_method(&method)
        .expect("Class.newInstance contract method should resolve")
        .expect("Class.newInstance contract method should exist");

    program
        .execute_method(None, &resolved, None, 0)
        .expect("Class.newInstance contract method should execute")
}

fn execute_class_new_instance_contract_void_method(
    program: &DexProgram,
    name: &str,
) -> DexExecutionResult {
    let method = DexMethodId {
        class_descriptor: "Lwinedroid/tests/ClassNewInstanceContract;".to_owned(),
        name: name.to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let resolved = program
        .resolve_exact_method(&method)
        .expect("void contract method should resolve")
        .expect("void contract method should exist");

    program
        .execute_method(None, &resolved, None, 0)
        .expect("void contract method should execute")
}

fn contract_static_int(program: &DexProgram, class_descriptor: &str, field_name: &str) -> i32 {
    let loaded = program
        .load_class(class_descriptor)
        .expect("contract state class");
    let encoded_field = loaded
        .class_def
        .static_fields
        .iter()
        .find(|encoded| {
            program
                .field_id(loaded.dex_file_index, encoded.field_index)
                .is_ok_and(|field| field.name == field_name)
        })
        .expect("contract state field");
    let field = program
        .field_id(loaded.dex_file_index, encoded_field.field_index)
        .expect("contract state field id");
    let Some(DexValue::Int(value)) = program.read_stored_static_field(&field) else {
        panic!("contract state field should contain an int");
    };
    value
}

fn resume_parked_contract_and_read(
    program: &DexProgram,
    method_name: &str,
    state_class_descriptor: &str,
) -> i32 {
    let result = execute_class_new_instance_contract_void_method(program, method_name);
    let DexExecutionResult::Parked(execution) = result else {
        panic!("contract method should suspend, got {result:?}");
    };
    program
        .resume_parked_thread(
            DexValue::Reference(std::rc::Rc::from("Ljava/lang/Thread;:main")),
            execution,
            0,
        )
        .expect("contract method should resume");
    contract_static_int(program, state_class_descriptor, "result")
}

// cite: art class_linker.cc#ClassLinker::InitializeClass
#[test]
fn art_encoded_static_values_overwrite_reentrant_parent_writes() {
    let program = class_new_instance_contract_program();
    let class_descriptor = "Lwinedroid/tests/EncodedStaticTarget;";
    assert_eq!(
        program.begin_class_initialization(class_descriptor),
        ClassInitializationStart::Started
    );
    let loaded = program.load_class(class_descriptor).expect("target class");
    let encoded_field = loaded
        .class_def
        .static_fields
        .iter()
        .find(|encoded| {
            program
                .field_id(loaded.dex_file_index, encoded.field_index)
                .is_ok_and(|field| field.name == "VALUE")
        })
        .expect("encoded VALUE field");
    let field = program
        .field_id(loaded.dex_file_index, encoded_field.field_index)
        .expect("VALUE field id");

    program.write_stored_static_field(&field, DexValue::Int(99));
    program.seed_encoded_static_values(class_descriptor);

    assert_eq!(
        program.read_stored_static_field(&field),
        Some(DexValue::Int(7))
    );
}

// cite: art java_lang_Class.cc#Class_newInstance
// cite: art class_linker.cc#ClassLinker::InitializeClass, ClassLinker::InitializeDefaultInterfaceRecursive
// cite: libcore Class.java#newInstance
#[test]
fn art_class_new_instance_initializes_class_and_executes_accessible_zero_argument_constructor() {
    for (method, expected) in [
        ("accessibleConstructorAndInitialization", 42),
        ("defaultInterfaceInitialization", 46),
        ("reentrantInitializationOrder", 12),
        ("encodedStaticValuesFollowParentInitialization", 23),
        ("priorStaticUsePreservesParentInitialization", 29),
    ] {
        assert_eq!(
            execute_class_new_instance_contract_method(method),
            DexExecutionResult::Int(expected),
            "{method}"
        );
    }
}

// cite: libcore Class.java#newInstance
// cite: libcore LockSupport.java#parkNanos
#[test]
fn libcore_class_new_instance_returns_allocated_receiver_after_parked_constructor_resumes() {
    let program = class_new_instance_contract_program();
    let result =
        execute_class_new_instance_contract_void_method(&program, "parkedConstructorCompletes");
    let DexExecutionResult::Parked(execution) = result else {
        panic!("parked constructor should suspend, got {result:?}");
    };

    program
        .resume_parked_thread(
            DexValue::Reference(std::rc::Rc::from("Ljava/lang/Thread;:main")),
            execution,
            0,
        )
        .expect("parked constructor should resume");

    assert_eq!(
        contract_static_int(
            &program,
            "Lwinedroid/tests/ParkedConstructorState;",
            "value"
        ),
        31
    );
}

// cite: art class_linker.cc#ClassLinker::InitializeClass
// cite: libcore Class.java#newInstance
// cite: libcore LockSupport.java#parkNanos
#[test]
fn art_class_new_instance_resumes_superclass_initialization_before_constructor() {
    let program = class_new_instance_contract_program();
    let result = execute_class_new_instance_contract_void_method(
        &program,
        "parkedClassInitializationCompletes",
    );
    let DexExecutionResult::Parked(execution) = result else {
        panic!("parked class initializer should suspend, got {result:?}");
    };

    program
        .resume_parked_thread(
            DexValue::Reference(std::rc::Rc::from("Ljava/lang/Thread;:main")),
            execution,
            0,
        )
        .expect("parked class initializer should resume");

    assert_eq!(
        contract_static_int(
            &program,
            "Lwinedroid/tests/ParkedInitializationState;",
            "result"
        ),
        31
    );
}

// cite: art nterp.cc#NterpGetStaticField
// cite: art class_linker.cc#ClassLinker::InitializeClass
// cite: libcore Class.java#newInstance
// cite: libcore LockSupport.java#parkNanos
#[test]
fn art_class_new_instance_resumes_nested_static_field_initialization_before_field_access() {
    for (method, state, expected) in [
        (
            "nestedParkedStaticReadCompletes",
            "Lwinedroid/tests/NestedParkedReadState;",
            31,
        ),
        (
            "nestedParkedStaticWriteCompletes",
            "Lwinedroid/tests/NestedParkedWriteState;",
            41,
        ),
    ] {
        let program = class_new_instance_contract_program();
        assert_eq!(
            resume_parked_contract_and_read(&program, method, state),
            expected,
            "{method}"
        );
    }
}

// cite: art nterp.cc#NterpGetStaticField
// cite: art interpreter_common.cc#MoveToExceptionHandler
// cite: art class_linker.cc#ClassLinker::InitializeClass
#[test]
fn art_parked_static_field_initialization_throw_resumes_at_the_field_handler() {
    let program = class_new_instance_contract_program();
    assert_eq!(
        resume_parked_contract_and_read(
            &program,
            "nestedParkedStaticReadThrowIsCaught",
            "Lwinedroid/tests/NestedParkedThrowState;",
        ),
        43
    );
}

// cite: art class_linker.cc#ClassLinker::InitializeClass, ClassLinker::WaitForInitializeClass
#[test]
fn art_class_initialization_is_reentrant_only_for_the_owning_guest_thread() {
    let program = class_new_instance_contract_program();
    let class_descriptor = "Lwinedroid/tests/NestedParkedReadTarget;";
    assert_eq!(
        program.begin_class_initialization(class_descriptor),
        ClassInitializationStart::Started
    );
    assert_eq!(
        program.begin_class_initialization(class_descriptor),
        ClassInitializationStart::Complete
    );

    let worker = DexValue::Reference(std::rc::Rc::from("Ljava/lang/Thread;:clinit-waiter"));
    let waiting = program.with_current_guest_thread(worker.clone(), || {
        program.begin_class_initialization(class_descriptor)
    });
    let ClassInitializationStart::Wait(execution) = waiting else {
        panic!("other guest thread should wait, got {waiting:?}");
    };
    assert_eq!(execution.waiting_class_descriptor(), Some(class_descriptor));
    program
        .runtime
        .borrow_mut()
        .store_parked_thread(&worker, execution)
        .expect("class initialization waiter should be stored");
    assert_eq!(
        program
            .runtime
            .borrow()
            .class_initialization_waiters
            .get(class_descriptor)
            .map(Vec::len),
        Some(1)
    );

    program.finish_class_initialization(class_descriptor);

    let runtime = program.runtime.borrow();
    assert!(runtime
        .class_initialization_waiters
        .get(class_descriptor)
        .is_none());
    assert_eq!(runtime.thread_resumes.len(), 1);
    assert_eq!(runtime.thread_resumes[0].thread, worker);
}

// cite: art class_linker.cc#ClassLinker::InitializeClass
// cite: art nterp.cc#NterpGetStaticField
#[test]
fn art_parked_class_initialization_host_error_clears_all_continuations() {
    let mut program = class_new_instance_contract_program();
    program.strict_missing = true;
    let result = execute_class_new_instance_contract_void_method(
        &program,
        "nestedParkedInitializerHostError",
    );
    let DexExecutionResult::Parked(execution) = result else {
        panic!("host-error initializer should suspend first, got {result:?}");
    };

    let error = program
        .resume_parked_thread(
            DexValue::Reference(std::rc::Rc::from("Ljava/lang/Thread;:main")),
            execution,
            0,
        )
        .expect_err("missing native primitive should remain an explicit host error");
    assert!(
        error.to_string().contains("missingRuntimePrimitive()V"),
        "{error}"
    );

    let runtime = program.runtime.borrow();
    assert!(runtime.initializing_classes.is_empty());
    assert!(runtime.parked_runtime_actions.is_empty());
    assert!(!runtime
        .initialized_classes
        .contains("Lwinedroid/tests/NestedParkedHostErrorHelper;"));
    assert!(!runtime
        .initialized_classes
        .contains("Lwinedroid/tests/NestedParkedHostErrorTarget;"));
}

// cite: art java_lang_Class.cc#Class_newInstance
// cite: libcore Class.java#newInstance
// cite: frameworks-base CharacterStyle.java#CharacterStyle
#[test]
fn art_class_new_instance_reports_instantiation_exception_for_uninstantiable_types() {
    for (method, expected) in [
        ("noZeroArgumentConstructor", 1),
        ("interfaceType", 2),
        ("abstractType", 3),
        ("arrayType", 4),
        ("primitiveType", 5),
        ("voidType", 15),
        ("bootstrapAbstractType", 20),
        ("bootstrapFrameworkAbstractType", 27),
        ("bootstrapMissingZeroArgumentConstructor", 21),
    ] {
        assert_eq!(
            execute_class_new_instance_contract_method(method),
            DexExecutionResult::Int(expected),
            "{method}"
        );
    }
}

// cite: art java_lang_Class.cc#Class_newInstance
// cite: art reflection.cc#VerifyAccess
// cite: art class-inl.h#Class::CanAccess
#[test]
fn art_class_new_instance_checks_class_and_constructor_access_against_caller() {
    for (method, expected) in [
        ("privateConstructor", 6),
        ("nonPublicCrossPackageClass", 7),
        ("samePackageClassAndConstructor", 44),
        ("privateConstructorFromDeclaringClass", 45),
    ] {
        assert_eq!(
            execute_class_new_instance_contract_method(method),
            DexExecutionResult::Int(expected),
            "{method}"
        );
    }
}

// cite: libcore Class.java#newInstance
#[test]
fn libcore_class_new_instance_propagates_constructor_exception_directly() {
    assert_eq!(
        execute_class_new_instance_contract_method("constructorExceptionPropagatesDirectly"),
        DexExecutionResult::Int(8)
    );
}

// cite: libcore Class.java#forName
#[test]
fn libcore_class_for_name_reports_missing_class_as_class_not_found_exception() {
    assert_eq!(
        execute_class_new_instance_contract_method("missingClassForName"),
        DexExecutionResult::Int(9)
    );
}

// cite: libcore InstantiationException.java#InstantiationException
// cite: libcore IllegalAccessException.java#IllegalAccessException
// cite: libcore ClassNotFoundException.java#ClassNotFoundException
// cite: libcore ReflectiveOperationException.java#ReflectiveOperationException
#[test]
fn libcore_reflective_failures_are_caught_as_reflective_operation_exception() {
    for (method, expected) in [
        ("reflectiveOperationExceptionHierarchy", 10),
        ("illegalAccessReflectiveOperationHierarchy", 13),
        ("classNotFoundReflectiveOperationHierarchy", 14),
    ] {
        assert_eq!(
            execute_class_new_instance_contract_method(method),
            DexExecutionResult::Int(expected),
            "{method}"
        );
    }
}

// cite: art java_lang_Class.cc#Class_newInstance
// cite: libcore ThreadLocal.java#ThreadLocal
// cite: libcore Date.java#Date
// cite: libcore Random.java#Random
// cite: libcore StringWriter.java#StringWriter
// cite: libcore AtomicBoolean.java#AtomicBoolean
// cite: libcore AtomicInteger.java#AtomicInteger
// cite: libcore AtomicReference.java#AtomicReference
// cite: frameworks-base Rect.java#Rect
// cite: frameworks-base RectF.java#RectF
// cite: frameworks-base Matrix.java#Matrix
// cite: frameworks-base Paint.java#Paint
// cite: frameworks-base Point.java#Point
// cite: libcore DefaultHandler.java#DefaultHandler
#[test]
fn art_class_new_instance_handles_empty_string_and_bootstrap_no_argument_constructors() {
    for (method, expected) in [
        ("emptyStringSpecialCase", 11),
        ("bootstrapNoArgumentConstructors", 12),
        ("bootstrapArrayListConstructor", 17),
        ("sourceBackedBootstrapConstructors", 28),
    ] {
        assert_eq!(
            execute_class_new_instance_contract_method(method),
            DexExecutionResult::Int(expected),
            "{method}"
        );
    }
}

// cite: art entrypoint_utils-inl.h#FindMethodToCall
#[test]
fn art_invoke_virtual_throws_null_pointer_exception_for_null_class_receiver() {
    assert_eq!(
        execute_class_new_instance_contract_method("nullClassReceiver"),
        DexExecutionResult::Int(16)
    );
}

// cite: art java_lang_Class.cc#Class_classForName
#[test]
fn art_class_for_name_resolves_bootstrap_classes_and_honors_initialization_flag() {
    for (method, expected) in [
        ("bootstrapClassForName", 18),
        ("classForNameInitializationControl", 19),
        ("nullLoaderUsesBootstrapLoader", 26),
    ] {
        assert_eq!(
            execute_class_new_instance_contract_method(method),
            DexExecutionResult::Int(expected),
            "{method}"
        );
    }
}

// cite: art java_lang_Class.cc#Class_classForName
// cite: art descriptors_names.cc#IsValidBinaryClassName, DotToDescriptor
#[test]
fn art_class_for_name_validates_and_converts_binary_names() {
    for (method, expected) in [
        ("classForNameArrayBinaryNames", 24),
        ("classForNameRejectsInvalidAndMissingArrayNames", 25),
    ] {
        assert_eq!(
            execute_class_new_instance_contract_method(method),
            DexExecutionResult::Int(expected),
            "{method}"
        );
    }
}

// cite: art java_lang_Class.cc#Class_classForName
#[test]
fn art_class_for_name_throws_null_pointer_exception_for_null_name() {
    assert_eq!(
        execute_class_new_instance_contract_method("nullClassForName"),
        DexExecutionResult::Int(22)
    );
}
