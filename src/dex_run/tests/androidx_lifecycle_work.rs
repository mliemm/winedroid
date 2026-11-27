use super::super::*;
use super::helpers::*;

#[test]
fn builtin_lifecycle_event_get_target_state_maps_on_create_to_created() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroidx/lifecycle/Lifecycle$Event;".to_owned(),
        name: "getTargetState".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroidx/lifecycle/Lifecycle$State;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let invocation = DexInvocation {
        kind: DexInvokeKind::Virtual,
        range: false,
        method_index: 0,
        argument_registers: smallvec::smallvec![0],
        arguments: smallvec::smallvec![DexValue::Reference(std::rc::Rc::from(
            "Landroidx/lifecycle/Lifecycle$Event;.ON_CREATE",
        ))],
    };

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Landroidx/lifecycle/Lifecycle$State;.CREATED")
        )))
    );
}

#[test]
fn builtin_obfuscated_lifecycle_event_target_state_maps_reached_events() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroidx/lifecycle/k;".to_owned(),
        name: "a".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroidx/lifecycle/l;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let on_create = test_invocation(vec![DexValue::Reference(std::rc::Rc::from(
        "Landroidx/lifecycle/k;.ON_CREATE",
    ))]);
    let on_destroy = test_invocation(vec![DexValue::Reference(std::rc::Rc::from(
        "Landroidx/lifecycle/k;.ON_DESTROY",
    ))]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&on_create)),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Landroidx/lifecycle/l;.c:Landroidx/lifecycle/l;")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&on_destroy)),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Landroidx/lifecycle/l;.a:Landroidx/lifecycle/l;")
        )))
    );
}

#[test]
fn builtin_lifecycle_registry_handle_event_returns_void() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroidx/lifecycle/LifecycleRegistry;".to_owned(),
        name: "handleLifecycleEvent".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroidx/lifecycle/Lifecycle$Event;".to_owned()],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&method, None),
        Some(DexExecutionResult::Void)
    );
}

#[test]
fn builtin_lifecycle_add_observer_returns_void() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroidx/lifecycle/Lifecycle;".to_owned(),
        name: "addObserver".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroidx/lifecycle/LifecycleObserver;".to_owned()],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&method, None),
        Some(DexExecutionResult::Void)
    );
}

#[test]
fn builtin_lifecycle_scope_helpers_return_bootstrap_scope() {
    let program = DexProgram::new(Vec::new());
    let owner_scope = DexMethodId {
        class_descriptor: "Landroidx/lifecycle/LifecycleOwnerKt;".to_owned(),
        name: "getLifecycleScope".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Landroidx/lifecycle/LifecycleCoroutineScope;".to_owned(),
            parameters: vec!["Landroidx/lifecycle/LifecycleOwner;".to_owned()],
        },
    };
    let lifecycle_scope = DexMethodId {
        class_descriptor: "Landroidx/lifecycle/LifecycleKt;".to_owned(),
        name: "getCoroutineScope".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Landroidx/lifecycle/LifecycleCoroutineScope;".to_owned(),
            parameters: vec!["Landroidx/lifecycle/Lifecycle;".to_owned()],
        },
    };
    let expected = Some(DexExecutionResult::Value(DexValue::Reference(
        std::rc::Rc::from("Landroidx/lifecycle/LifecycleCoroutineScope;.bootstrap"),
    )));

    assert_eq!(program.invoke_builtin_method(&owner_scope, None), expected);
    assert_eq!(
        program.invoke_builtin_method(&lifecycle_scope, None),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Landroidx/lifecycle/LifecycleCoroutineScope;.bootstrap")
        )))
    );
}

#[test]
fn builtin_work_manager_get_instance_returns_bootstrap_reference() {
    let program = DexProgram::new(Vec::new());
    let get_instance = DexMethodId {
        class_descriptor: "Landroidx/work/WorkManager$Companion;".to_owned(),
        name: "getInstance".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Landroidx/work/WorkManager;".to_owned(),
            parameters: vec!["Landroid/content/Context;".to_owned()],
        },
    };
    let get_by_tag = DexMethodId {
        class_descriptor: "Landroidx/work/WorkManager;".to_owned(),
        name: "getWorkInfosByTag".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Lcom/google/common/util/concurrent/ListenableFuture;".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned()],
        },
    };
    let future_get = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/Future;".to_owned(),
        name: "get".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let list_iterator = DexMethodId {
        class_descriptor: "Ljava/util/List;".to_owned(),
        name: "iterator".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/util/Iterator;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get_by_tag_invocation = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from("Landroidx/work/WorkManager;.bootstrap")),
        DexValue::String(std::rc::Rc::from("download")),
    ]);
    let future_get_invocation = test_invocation(vec![DexValue::Reference(std::rc::Rc::from(
        "Lcom/google/common/util/concurrent/ListenableFuture;:workInfosByTag:download",
    ))]);
    let list_iterator_invocation = test_invocation(vec![DexValue::Reference(std::rc::Rc::from(
        "Ljava/util/List;:workInfosByTag:download:empty",
    ))]);

    assert_eq!(
        program.invoke_builtin_method(&get_instance, None),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Landroidx/work/WorkManager;.bootstrap")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&get_by_tag, Some(&get_by_tag_invocation)),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from(
                "Lcom/google/common/util/concurrent/ListenableFuture;:workInfosByTag:download"
                    .to_owned()
            )
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&future_get, Some(&future_get_invocation)),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Ljava/util/List;:workInfosByTag:download:empty")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&list_iterator, Some(&list_iterator_invocation)),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Ljava/util/Iterator;.empty")
        )))
    );
}

#[test]
fn builtin_work_manager_enqueue_unique_work_list_returns_operation() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroidx/work/WorkManager;".to_owned(),
        name: "enqueueUniqueWork".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LLLL".to_owned(),
            return_type: "Landroidx/work/Operation;".to_owned(),
            parameters: vec![
                "Ljava/lang/String;".to_owned(),
                "Landroidx/work/ExistingWorkPolicy;".to_owned(),
                "Ljava/util/List;".to_owned(),
            ],
        },
    };
    let invocation = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from("Landroidx/work/WorkManager;.bootstrap")),
        DexValue::String(std::rc::Rc::from("downloadApkWorker")),
        DexValue::Reference(std::rc::Rc::from("Landroidx/work/ExistingWorkPolicy;.KEEP")),
        DexValue::Reference(std::rc::Rc::from(
            "Ljava/util/List;:collections:singletonList:1",
        )),
    ]);

    assert_eq!(
            program.invoke_builtin_method(&method, Some(&invocation)),
            Some(DexExecutionResult::Value(DexValue::Reference(
                std::rc::Rc::from("Landroidx/work/Operation;:enqueueUniqueWork:downloadApkWorker:reference:Ljava/util/List;:collections:singletonList:1")
            )))
        );
}

#[test]
fn builtin_work_request_builder_get_this_object_returns_receiver() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroidx/work/WorkRequest$Builder;".to_owned(),
        name: "getThisObject$work_runtime_release".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroidx/work/WorkRequest$Builder;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let builder = DexValue::Object(DexObjectRef {
        class_descriptor: std::rc::Rc::from("Landroidx/work/OneTimeWorkRequest$Builder;"),
        id: 1,
    });
    let invocation = test_invocation(vec![builder.clone()]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Value(builder))
    );
}

#[test]
fn builtin_work_request_builder_build_internal_returns_built_request() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroidx/work/WorkRequest$Builder;".to_owned(),
        name: "buildInternal$work_runtime_release".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroidx/work/WorkRequest;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let builder = DexValue::Object(DexObjectRef {
        class_descriptor: std::rc::Rc::from("Landroidx/work/OneTimeWorkRequest$Builder;"),
        id: 7,
    });
    let invocation = test_invocation(vec![builder]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Landroidx/work/WorkRequest;:built:object:7")
        )))
    );
}

#[test]
fn builtin_lifecycle_stubs_defer_to_guest_receivers() {
    let program = DexProgram::new(Vec::new());
    let method = |class: &str, name: &str, ret: &str, params: Vec<String>| DexMethodId {
        class_descriptor: class.to_owned(),
        name: name.to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: ret.to_owned(),
            parameters: params,
        },
    };
    let get_lifecycle = method(
        "Landroidx/lifecycle/LifecycleOwner;",
        "getLifecycle",
        "Landroidx/lifecycle/Lifecycle;",
        Vec::new(),
    );
    let add_observer = method(
        "Landroidx/lifecycle/Lifecycle;",
        "addObserver",
        "V",
        vec!["Landroidx/lifecycle/LifecycleObserver;".to_owned()],
    );
    let owner = program.allocate_object("Lcom/example/MainActivity;".to_owned());
    let registry = program.allocate_object("Landroidx/lifecycle/LifecycleRegistry;".to_owned());
    let observer = program.allocate_object("Lcom/example/Observer;".to_owned());

    assert_eq!(
        program.invoke_builtin_method(&get_lifecycle, Some(&test_invocation(vec![owner]))),
        None
    );
    assert_eq!(
        program.invoke_builtin_method(
            &add_observer,
            Some(&test_invocation(vec![registry, observer]))
        ),
        None
    );
}
