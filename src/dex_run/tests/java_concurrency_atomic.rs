use super::super::*;
use super::helpers::*;

#[test]
fn builtin_atomic_boolean_stores_and_updates_state() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/atomic/AtomicBoolean;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VZ".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Z".to_owned()],
        },
    };
    let get = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/atomic/AtomicBoolean;".to_owned(),
        name: "get".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/atomic/AtomicBoolean;".to_owned(),
        name: "set".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VZ".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Z".to_owned()],
        },
    };
    let get_and_set = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/atomic/AtomicBoolean;".to_owned(),
        name: "getAndSet".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZZ".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Z".to_owned()],
        },
    };
    let compare_and_set = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/atomic/AtomicBoolean;".to_owned(),
        name: "compareAndSet".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZZZ".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Z".to_owned(), "Z".to_owned()],
        },
    };
    let object = DexValue::Reference(std::rc::Rc::from("atomic-bool"));

    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![object.clone(), DexValue::Int(1)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&get, Some(&test_invocation(vec![object.clone()]))),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set,
            Some(&test_invocation(vec![object.clone(), DexValue::Int(0)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &compare_and_set,
            Some(&test_invocation(vec![
                object.clone(),
                DexValue::Int(1),
                DexValue::Int(1),
            ]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &compare_and_set,
            Some(&test_invocation(vec![
                object.clone(),
                DexValue::Int(0),
                DexValue::Int(1),
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_and_set,
            Some(&test_invocation(vec![object.clone(), DexValue::Int(0)]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&get, Some(&test_invocation(vec![object]))),
        Some(DexExecutionResult::Int(0))
    );
}

#[test]
fn builtin_executors_new_fixed_thread_pool_returns_executor_service() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/Executors;".to_owned(),
        name: "newFixedThreadPool".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LIL".to_owned(),
            return_type: "Ljava/util/concurrent/ExecutorService;".to_owned(),
            parameters: vec![
                "I".to_owned(),
                "Ljava/util/concurrent/ThreadFactory;".to_owned(),
            ],
        },
    };
    let invocation = test_invocation(vec![
        DexValue::Int(2),
        DexValue::Reference(std::rc::Rc::from("thread-factory")),
    ]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Ljava/util/concurrent/ExecutorService;.fixed/2")
        )))
    );
}

#[test]
fn builtin_executors_new_cached_thread_pool_returns_executor_service() {
    // cite: libcore Executors.java#newCachedThreadPool
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/Executors;".to_owned(),
        name: "newCachedThreadPool".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/util/concurrent/ExecutorService;".to_owned(),
            parameters: Vec::new(),
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(Vec::new()))),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Ljava/util/concurrent/ExecutorService;.cached")
        )))
    );
}

#[test]
fn builtin_future_task_callable_constructor_stores_callable() {
    // cite: libcore FutureTask.java#FutureTask
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/FutureTask;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/util/concurrent/Callable;".to_owned()],
        },
    };
    let task = DexValue::Object(DexObjectRef {
        id: 1,
        class_descriptor: std::rc::Rc::from("Ljava/util/concurrent/FutureTask;"),
    });
    let callable = DexValue::Object(DexObjectRef {
        id: 2,
        class_descriptor: std::rc::Rc::from("Lexample/ListFilesCallable;"),
    });
    let callable_field = DexFieldId {
        class_descriptor: "Ljava/util/concurrent/FutureTask;".to_owned(),
        type_descriptor: "Ljava/util/concurrent/Callable;".to_owned(),
        name: "callable".to_owned(),
    };

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![task.clone(), callable.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&task, &callable_field),
        Some(callable)
    );
}

#[test]
fn builtin_future_task_runnable_constructor_stores_runnable_and_fixed_result() {
    // cite: libcore FutureTask.java#FutureTask
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/FutureTask;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VLL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "Ljava/lang/Runnable;".to_owned(),
                "Ljava/lang/Object;".to_owned(),
            ],
        },
    };
    let task = DexValue::Object(DexObjectRef {
        id: 3,
        class_descriptor: std::rc::Rc::from("Ljava/util/concurrent/FutureTask;"),
    });
    let runnable = DexValue::Object(DexObjectRef {
        id: 4,
        class_descriptor: std::rc::Rc::from("Ljava/util/concurrent/FutureTask;"),
    });
    let runnable_field = DexFieldId {
        class_descriptor: "Ljava/util/concurrent/FutureTask;".to_owned(),
        type_descriptor: "Ljava/lang/Runnable;".to_owned(),
        name: "runnable".to_owned(),
    };
    let result_field = DexFieldId {
        class_descriptor: "Ljava/util/concurrent/FutureTask;".to_owned(),
        type_descriptor: "Ljava/lang/Object;".to_owned(),
        name: "runnableResult".to_owned(),
    };

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                task.clone(),
                runnable.clone(),
                DexValue::Int(0),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&task, &runnable_field),
        Some(runnable)
    );
    assert_eq!(
        program.read_stored_instance_field(&task, &result_field),
        Some(DexValue::Int(0))
    );
}

#[test]
fn builtin_future_task_cancel_marks_incomplete_task_cancelled_and_done() {
    // cite: libcore FutureTask.java#cancel
    let program = DexProgram::new(Vec::new());
    let cancel = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/FutureTask;".to_owned(),
        name: "cancel".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZZ".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Z".to_owned()],
        },
    };
    let is_cancelled = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/FutureTask;".to_owned(),
        name: "isCancelled".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let is_done = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/FutureTask;".to_owned(),
        name: "isDone".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let task = DexValue::Object(DexObjectRef {
        id: 5,
        class_descriptor: std::rc::Rc::from("Ljava/util/concurrent/FutureTask;"),
    });
    let task_invocation = test_invocation(vec![task.clone()]);

    assert_eq!(
        program.invoke_builtin_method(&is_cancelled, Some(&task_invocation)),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&is_done, Some(&task_invocation)),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &cancel,
            Some(&test_invocation(vec![task.clone(), DexValue::Int(1)]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&is_cancelled, Some(&task_invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&is_done, Some(&task_invocation)),
        Some(DexExecutionResult::Int(1))
    );
}

#[test]
fn builtin_future_task_cancel_returns_false_for_an_already_done_task() {
    // cite: libcore FutureTask.java#cancel
    let program = DexProgram::new(Vec::new());
    let cancel = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/FutureTask;".to_owned(),
        name: "cancel".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZZ".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Z".to_owned()],
        },
    };
    let is_cancelled = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/FutureTask;".to_owned(),
        name: "isCancelled".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let task = DexValue::Object(DexObjectRef {
        id: 6,
        class_descriptor: std::rc::Rc::from("Ljava/util/concurrent/FutureTask;"),
    });
    let done_field = DexFieldId {
        class_descriptor: "Ljava/util/concurrent/FutureTask;".to_owned(),
        type_descriptor: "Z".to_owned(),
        name: "done".to_owned(),
    };
    assert!(program.write_stored_instance_field(&task, &done_field, DexValue::Int(1)));

    assert_eq!(
        program.invoke_builtin_method(
            &cancel,
            Some(&test_invocation(vec![task.clone(), DexValue::Int(0)]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&is_cancelled, Some(&test_invocation(vec![task]))),
        Some(DexExecutionResult::Int(0))
    );
}

#[test]
fn future_task_get_runs_an_unstarted_callable_task_inline_and_returns_its_outcome() {
    // cite: libcore FutureTask.java#get, report
    let program = DexProgram::new(Vec::new());
    let task = DexValue::Object(DexObjectRef {
        id: 10,
        class_descriptor: std::rc::Rc::from("Ljava/util/concurrent/FutureTask;"),
    });
    let outcome_field = DexFieldId {
        class_descriptor: "Ljava/util/concurrent/FutureTask;".to_owned(),
        type_descriptor: "Ljava/lang/Object;".to_owned(),
        name: "outcome".to_owned(),
    };
    let done_field = DexFieldId {
        class_descriptor: "Ljava/util/concurrent/FutureTask;".to_owned(),
        type_descriptor: "Z".to_owned(),
        name: "done".to_owned(),
    };
    assert!(program.write_stored_instance_field(&task, &done_field, DexValue::Int(1)));
    let value = DexValue::Reference(std::rc::Rc::from("result"));
    assert!(program.write_stored_instance_field(&task, &outcome_field, value.clone()));

    let invocation = test_invocation(vec![task]);
    assert_eq!(
        program.future_task_get(&invocation, 0).unwrap(),
        DexExecutionResult::Value(value)
    );
}

#[test]
fn future_task_get_throws_execution_exception_wrapping_a_stored_failure() {
    // cite: libcore FutureTask.java#report
    let program = DexProgram::new(Vec::new());
    let task = DexValue::Object(DexObjectRef {
        id: 11,
        class_descriptor: std::rc::Rc::from("Ljava/util/concurrent/FutureTask;"),
    });
    let done_field = DexFieldId {
        class_descriptor: "Ljava/util/concurrent/FutureTask;".to_owned(),
        type_descriptor: "Z".to_owned(),
        name: "done".to_owned(),
    };
    let exception_field = DexFieldId {
        class_descriptor: "Ljava/util/concurrent/FutureTask;".to_owned(),
        type_descriptor: "Ljava/lang/Throwable;".to_owned(),
        name: "outcomeException".to_owned(),
    };
    let cause = DexValue::Object(DexObjectRef {
        id: 12,
        class_descriptor: std::rc::Rc::from("Ljava/io/IOException;"),
    });
    assert!(program.write_stored_instance_field(&task, &done_field, DexValue::Int(1)));
    assert!(program.write_stored_instance_field(&task, &exception_field, cause.clone()));

    let invocation = test_invocation(vec![task]);
    let DexExecutionResult::Throw(exception) = program.future_task_get(&invocation, 0).unwrap()
    else {
        panic!("expected FutureTask.get to throw when outcomeException is set");
    };
    assert_eq!(
        value_class_descriptor(&exception),
        Some("Ljava/util/concurrent/ExecutionException;")
    );
    let cause_field = DexFieldId {
        class_descriptor: "Ljava/lang/Throwable;".to_owned(),
        type_descriptor: "Ljava/lang/Throwable;".to_owned(),
        name: "cause".to_owned(),
    };
    assert_eq!(
        program.read_stored_instance_field(&exception, &cause_field),
        Some(cause)
    );
}

#[test]
fn future_task_get_throws_cancellation_exception_for_a_cancelled_task() {
    // cite: libcore FutureTask.java#report
    let program = DexProgram::new(Vec::new());
    let task = DexValue::Object(DexObjectRef {
        id: 13,
        class_descriptor: std::rc::Rc::from("Ljava/util/concurrent/FutureTask;"),
    });
    let cancel = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/FutureTask;".to_owned(),
        name: "cancel".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZZ".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Z".to_owned()],
        },
    };
    assert_eq!(
        program.invoke_builtin_method(
            &cancel,
            Some(&test_invocation(vec![task.clone(), DexValue::Int(0)]))
        ),
        Some(DexExecutionResult::Int(1))
    );

    let invocation = test_invocation(vec![task]);
    let DexExecutionResult::Throw(exception) = program.future_task_get(&invocation, 0).unwrap()
    else {
        panic!("expected FutureTask.get to throw for a cancelled task");
    };
    assert_eq!(
        value_class_descriptor(&exception),
        Some("Ljava/util/concurrent/CancellationException;")
    );
}

#[test]
fn builtin_lock_support_park_nanos_accepts_wide_timeout_argument() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/locks/LockSupport;".to_owned(),
        name: "parkNanos".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VJ".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["J".to_owned()],
        },
    };
    let invocation = test_invocation(vec![DexValue::Long(1_000), DexValue::Long(1_000)]);

    assert!(matches!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Parked(_))
    ));
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![DexValue::Long(0), DexValue::Long(0)]))
        ),
        Some(DexExecutionResult::Void)
    );
}

#[test]
fn builtin_lock_support_park_nanos_with_blocker_accepts_nullable_blocker() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/locks/LockSupport;".to_owned(),
        name: "parkNanos".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VLJ".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned(), "J".to_owned()],
        },
    };
    let blocker = DexValue::Object(DexObjectRef {
        id: 43,
        class_descriptor: std::rc::Rc::from("Ljava/lang/Object;"),
    });

    assert!(matches!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                blocker,
                DexValue::Long(1_000),
                DexValue::Long(1_000),
            ]))
        ),
        Some(DexExecutionResult::Parked(_))
    ));
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::Null,
                DexValue::Long(0),
                DexValue::Long(0),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
}

#[test]
fn builtin_lock_support_unpark_accepts_thread_or_null() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/locks/LockSupport;".to_owned(),
        name: "unpark".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/lang/Thread;".to_owned()],
        },
    };
    let thread = DexValue::Object(DexObjectRef {
        id: 27,
        class_descriptor: std::rc::Rc::from("Ljava/lang/Thread;"),
    });

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![thread]))),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![DexValue::Null]))),
        Some(DexExecutionResult::Void)
    );
}

#[test]
fn builtin_lock_support_unpark_queues_parked_thread_resume() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/locks/LockSupport;".to_owned(),
        name: "unpark".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/lang/Thread;".to_owned()],
        },
    };
    let thread = DexValue::Object(DexObjectRef {
        id: 42,
        class_descriptor: std::rc::Rc::from("Ljava/lang/Thread;"),
    });
    program
        .runtime
        .borrow_mut()
        .store_parked_thread(&thread, DexParkedExecution::empty())
        .unwrap();

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![thread.clone()]))),
        Some(DexExecutionResult::Void)
    );

    let runtime = program.runtime.borrow();
    assert!(runtime.parked_threads.is_empty());
    assert_eq!(runtime.thread_resumes.len(), 1);
    assert_eq!(runtime.thread_resumes[0].thread, thread);
}

#[test]
fn lock_support_unpark_before_park_grants_a_permit() {
    // cite: libcore LockSupport.java#unpark
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/locks/LockSupport;".to_owned(),
        name: "unpark".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/lang/Thread;".to_owned()],
        },
    };
    let thread = DexValue::Object(DexObjectRef {
        id: 43,
        class_descriptor: std::rc::Rc::from("Ljava/lang/Thread;"),
    });

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![thread.clone()]))),
        Some(DexExecutionResult::Void)
    );

    program
        .runtime
        .borrow_mut()
        .store_parked_thread(&thread, DexParkedExecution::empty())
        .unwrap();

    let runtime = program.runtime.borrow();
    assert!(runtime.parked_threads.is_empty());
    assert_eq!(runtime.thread_resumes.len(), 1);
    assert_eq!(runtime.thread_resumes[0].thread, thread);
}

#[test]
fn timed_park_resumes_when_the_uptime_clock_reaches_the_deadline() {
    // cite: libcore LockSupport.java#parkNanos
    let program = DexProgram::new(Vec::new());
    let thread = DexValue::Object(DexObjectRef {
        id: 44,
        class_descriptor: std::rc::Rc::from("Ljava/lang/Thread;"),
    });

    program
        .runtime
        .borrow_mut()
        .store_parked_thread(&thread, DexParkedExecution::with_park_timeout(50_000_000))
        .unwrap();

    {
        let mut runtime = program.runtime.borrow_mut();
        runtime.resume_expired_parked_threads();
        assert!(runtime.thread_resumes.is_empty());

        runtime.main_thread_uptime_millis += 50;
        runtime.resume_expired_parked_threads();
        assert_eq!(runtime.thread_resumes.len(), 1);
        assert_eq!(runtime.thread_resumes[0].thread, thread);
        assert!(runtime.parked_threads.is_empty());
    }
}

#[test]
fn builtin_reentrant_lock_supports_single_threaded_lock_flow() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/locks/ReentrantLock;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let lock = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/locks/ReentrantLock;".to_owned(),
        name: "lock".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let try_lock = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/locks/ReentrantLock;".to_owned(),
        name: "tryLock".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let unlock = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/locks/ReentrantLock;".to_owned(),
        name: "unlock".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let object = DexValue::Reference(std::rc::Rc::from("lock"));

    assert_eq!(
        program.invoke_builtin_method(&constructor, Some(&test_invocation(vec![object.clone()]))),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&lock, Some(&test_invocation(vec![object.clone()]))),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&try_lock, Some(&test_invocation(vec![object.clone()]))),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&unlock, Some(&test_invocation(vec![object]))),
        Some(DexExecutionResult::Void)
    );
}

#[test]
fn builtin_atomic_integer_constructor_returns_void() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/atomic/AtomicInteger;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let increment_and_get = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/atomic/AtomicInteger;".to_owned(),
        name: "incrementAndGet".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get_and_increment = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/atomic/AtomicInteger;".to_owned(),
        name: "getAndIncrement".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let decrement_and_get = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/atomic/AtomicInteger;".to_owned(),
        name: "decrementAndGet".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/atomic/AtomicInteger;".to_owned(),
        name: "get".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/atomic/AtomicInteger;".to_owned(),
        name: "set".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let add_and_get = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/atomic/AtomicInteger;".to_owned(),
        name: "addAndGet".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "II".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let get_and_add = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/atomic/AtomicInteger;".to_owned(),
        name: "getAndAdd".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "II".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let compare_and_set = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/atomic/AtomicInteger;".to_owned(),
        name: "compareAndSet".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZII".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["I".to_owned(), "I".to_owned()],
        },
    };
    let invocation = test_invocation(vec![DexValue::Reference(std::rc::Rc::from("counter"))]);

    assert_eq!(
        program.invoke_builtin_method(&constructor, Some(&invocation)),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&increment_and_get, Some(&invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&increment_and_get, Some(&invocation)),
        Some(DexExecutionResult::Int(2))
    );
    assert_eq!(
        program.invoke_builtin_method(&get_and_increment, Some(&invocation)),
        Some(DexExecutionResult::Int(2))
    );
    assert_eq!(
        program.invoke_builtin_method(&decrement_and_get, Some(&invocation)),
        Some(DexExecutionResult::Int(2))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set,
            Some(&test_invocation(vec![
                DexValue::Reference(std::rc::Rc::from("counter")),
                DexValue::Int(10),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&get, Some(&invocation)),
        Some(DexExecutionResult::Int(10))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &add_and_get,
            Some(&test_invocation(vec![
                DexValue::Reference(std::rc::Rc::from("counter")),
                DexValue::Int(3),
            ]))
        ),
        Some(DexExecutionResult::Int(13))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_and_add,
            Some(&test_invocation(vec![
                DexValue::Reference(std::rc::Rc::from("counter")),
                DexValue::Int(2),
            ]))
        ),
        Some(DexExecutionResult::Int(13))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &compare_and_set,
            Some(&test_invocation(vec![
                DexValue::Reference(std::rc::Rc::from("counter")),
                DexValue::Int(15),
                DexValue::Int(20),
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&get, Some(&invocation)),
        Some(DexExecutionResult::Int(20))
    );
}

#[test]
fn builtin_atomic_integer_int_constructor_stores_initial_value() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/atomic/AtomicInteger;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let increment_and_get = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/atomic/AtomicInteger;".to_owned(),
        name: "incrementAndGet".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let invocation = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from("counter")),
        DexValue::Int(41),
    ]);
    let counter_invocation =
        test_invocation(vec![DexValue::Reference(std::rc::Rc::from("counter"))]);

    assert_eq!(
        program.invoke_builtin_method(&constructor, Some(&invocation)),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&increment_and_get, Some(&counter_invocation)),
        Some(DexExecutionResult::Int(42))
    );
}

#[test]
fn builtin_atomic_reference_constructor_stores_initial_value() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/atomic/AtomicReference;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let object = DexValue::Reference(std::rc::Rc::from("atomic-ref"));
    let value = DexValue::Reference(std::rc::Rc::from("initial"));
    let invocation = test_invocation(vec![object.clone(), value.clone()]);
    let null_object = DexValue::Reference(std::rc::Rc::from("atomic-null-ref"));
    let null_invocation = test_invocation(vec![null_object.clone(), DexValue::Int(0)]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&null_invocation)),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .atomic_reference_values
            .get(&map_identity_key(&object)),
        Some(&value)
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .atomic_reference_values
            .get(&map_identity_key(&null_object)),
        Some(&DexValue::Null)
    );
}

#[test]
fn builtin_atomic_reference_get_and_set_round_trip_values() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/atomic/AtomicReference;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/atomic/AtomicReference;".to_owned(),
        name: "get".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/atomic/AtomicReference;".to_owned(),
        name: "set".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let lazy_set = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/atomic/AtomicReference;".to_owned(),
        name: "lazySet".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let get_and_set = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/atomic/AtomicReference;".to_owned(),
        name: "getAndSet".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let compare_and_set = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/atomic/AtomicReference;".to_owned(),
        name: "compareAndSet".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZLL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec![
                "Ljava/lang/Object;".to_owned(),
                "Ljava/lang/Object;".to_owned(),
            ],
        },
    };
    let object = DexValue::Reference(std::rc::Rc::from("atomic-ref"));
    let object_invocation = test_invocation(vec![object.clone()]);
    let value = DexValue::Reference(std::rc::Rc::from("updated"));
    let next_value = DexValue::Reference(std::rc::Rc::from("next"));
    let final_value = DexValue::Reference(std::rc::Rc::from("final"));

    assert_eq!(
        program.invoke_builtin_method(&get, Some(&object_invocation)),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    assert_eq!(
        program.invoke_builtin_method(&constructor, Some(&object_invocation)),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&get, Some(&object_invocation)),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set,
            Some(&test_invocation(vec![object.clone(), value.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&get, Some(&object_invocation)),
        Some(DexExecutionResult::Value(value))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &lazy_set,
            Some(&test_invocation(vec![
                object.clone(),
                DexValue::Reference(std::rc::Rc::from("updated"))
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &compare_and_set,
            Some(&test_invocation(vec![
                object.clone(),
                DexValue::Null,
                next_value.clone()
            ]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &compare_and_set,
            Some(&test_invocation(vec![
                object.clone(),
                DexValue::Reference(std::rc::Rc::from("updated")),
                next_value.clone()
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&get, Some(&object_invocation)),
        Some(DexExecutionResult::Value(next_value))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_and_set,
            Some(&test_invocation(vec![object.clone(), final_value.clone()]))
        ),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("next")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&get, Some(&object_invocation)),
        Some(DexExecutionResult::Value(final_value))
    );
}

#[test]
fn builtin_atomic_reference_array_stores_slots() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/atomic/AtomicReferenceArray;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let length = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/atomic/AtomicReferenceArray;".to_owned(),
        name: "length".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/atomic/AtomicReferenceArray;".to_owned(),
        name: "get".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LI".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let set = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/atomic/AtomicReferenceArray;".to_owned(),
        name: "set".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VIL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned(), "Ljava/lang/Object;".to_owned()],
        },
    };
    let get_and_set = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/atomic/AtomicReferenceArray;".to_owned(),
        name: "getAndSet".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LIL".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: vec!["I".to_owned(), "Ljava/lang/Object;".to_owned()],
        },
    };
    let compare_and_set = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/atomic/AtomicReferenceArray;".to_owned(),
        name: "compareAndSet".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZILL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec![
                "I".to_owned(),
                "Ljava/lang/Object;".to_owned(),
                "Ljava/lang/Object;".to_owned(),
            ],
        },
    };
    let array = DexValue::Reference(std::rc::Rc::from("atomic-array"));
    let value = DexValue::Reference(std::rc::Rc::from("stored"));
    let next_value = DexValue::Reference(std::rc::Rc::from("next"));
    let final_value = DexValue::Reference(std::rc::Rc::from("final"));

    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![array.clone(), DexValue::Int(2)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&length, Some(&test_invocation(vec![array.clone()]))),
        Some(DexExecutionResult::Int(2))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get,
            Some(&test_invocation(vec![array.clone(), DexValue::Int(1)]))
        ),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set,
            Some(&test_invocation(vec![
                array.clone(),
                DexValue::Int(1),
                value.clone()
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get,
            Some(&test_invocation(vec![array.clone(), DexValue::Int(1)]))
        ),
        Some(DexExecutionResult::Value(value.clone()))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &compare_and_set,
            Some(&test_invocation(vec![
                array.clone(),
                DexValue::Int(1),
                DexValue::Null,
                next_value.clone()
            ]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &compare_and_set,
            Some(&test_invocation(vec![
                array.clone(),
                DexValue::Int(1),
                value,
                next_value.clone()
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&get, Some(&test_invocation(vec![array, DexValue::Int(1)]))),
        Some(DexExecutionResult::Value(next_value))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_and_set,
            Some(&test_invocation(vec![
                DexValue::Reference(std::rc::Rc::from("atomic-array")),
                DexValue::Int(1),
                final_value.clone()
            ]))
        ),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("next")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get,
            Some(&test_invocation(vec![
                DexValue::Reference(std::rc::Rc::from("atomic-array")),
                DexValue::Int(1)
            ]))
        ),
        Some(DexExecutionResult::Value(final_value))
    );
}

#[test]
fn builtin_atomic_reference_field_updater_get_reads_reached_state_field() {
    let program = DexProgram::new(Vec::new());
    let new_updater = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/atomic/AtomicReferenceFieldUpdater;".to_owned(),
        name: "newUpdater".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LLLL".to_owned(),
            return_type: "Ljava/util/concurrent/atomic/AtomicReferenceFieldUpdater;".to_owned(),
            parameters: vec![
                "Ljava/lang/Class;".to_owned(),
                "Ljava/lang/Class;".to_owned(),
                "Ljava/lang/String;".to_owned(),
            ],
        },
    };
    let get = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/atomic/AtomicReferenceFieldUpdater;".to_owned(),
        name: "get".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let set = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/atomic/AtomicReferenceFieldUpdater;".to_owned(),
        name: "set".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VLL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "Ljava/lang/Object;".to_owned(),
                "Ljava/lang/Object;".to_owned(),
            ],
        },
    };
    let lazy_set = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/atomic/AtomicReferenceFieldUpdater;".to_owned(),
        name: "lazySet".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VLL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "Ljava/lang/Object;".to_owned(),
                "Ljava/lang/Object;".to_owned(),
            ],
        },
    };
    let get_and_set = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/atomic/AtomicReferenceFieldUpdater;".to_owned(),
        name: "getAndSet".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LLL".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: vec![
                "Ljava/lang/Object;".to_owned(),
                "Ljava/lang/Object;".to_owned(),
            ],
        },
    };
    let compare_and_set = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/atomic/AtomicReferenceFieldUpdater;".to_owned(),
        name: "compareAndSet".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZLLL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec![
                "Ljava/lang/Object;".to_owned(),
                "Ljava/lang/Object;".to_owned(),
                "Ljava/lang/Object;".to_owned(),
            ],
        },
    };
    let holder = DexValue::Object(DexObjectRef {
        id: 1,
        class_descriptor: std::rc::Rc::from("LJ1/a;"),
    });
    let updater = DexValue::Reference(std::rc::Rc::from(
        "Ljava/util/concurrent/atomic/AtomicReferenceFieldUpdater;:LJ1/a;._state",
    ));
    let field = DexFieldId {
        class_descriptor: "LJ1/a;".to_owned(),
        type_descriptor: "Ljava/lang/Object;".to_owned(),
        name: "_state".to_owned(),
    };
    let value = DexValue::Reference(std::rc::Rc::from(
        "Landroidx/lifecycle/l;.b:Landroidx/lifecycle/l;",
    ));
    let next_value = DexValue::Reference(std::rc::Rc::from(
        "Landroidx/lifecycle/l;.c:Landroidx/lifecycle/l;",
    ));
    let Some(DexExecutionResult::Value(created_updater)) = program.invoke_builtin_method(
        &new_updater,
        Some(&test_invocation(vec![
            DexValue::Reference(std::rc::Rc::from("Ljava/lang/Class;:LJ1/a;")),
            DexValue::Reference(std::rc::Rc::from("Ljava/lang/Class;:Ljava/lang/Object;")),
            DexValue::String(std::rc::Rc::from("_state")),
        ])),
    ) else {
        panic!("AtomicReferenceFieldUpdater.newUpdater should return an updater reference");
    };
    let invocation = test_invocation(vec![updater.clone(), holder.clone()]);
    let created_invocation = test_invocation(vec![created_updater.clone(), holder.clone()]);
    let set_invocation = test_invocation(vec![created_updater, holder.clone(), next_value.clone()]);

    assert_eq!(
        program.invoke_builtin_method(&get, Some(&invocation)),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    assert!(program.write_stored_instance_field(&holder, &field, value.clone()));
    assert_eq!(
        program.invoke_builtin_method(&get, Some(&invocation)),
        Some(DexExecutionResult::Value(value.clone()))
    );
    assert_eq!(
        program.invoke_builtin_method(&get, Some(&created_invocation)),
        Some(DexExecutionResult::Value(value.clone()))
    );
    assert_eq!(
        program.invoke_builtin_method(&set, Some(&set_invocation)),
        Some(DexExecutionResult::Void)
    );
    // cite: libcore AtomicReferenceFieldUpdater.java#lazySet
    assert_eq!(
        program.invoke_builtin_method(
            &lazy_set,
            Some(&test_invocation(vec![
                updater.clone(),
                holder.clone(),
                next_value.clone(),
            ])),
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &compare_and_set,
            Some(&test_invocation(vec![
                updater.clone(),
                holder.clone(),
                value.clone(),
                DexValue::Null,
            ]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &compare_and_set,
            Some(&test_invocation(vec![
                updater.clone(),
                holder.clone(),
                next_value.clone(),
                value.clone(),
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&get, Some(&created_invocation)),
        Some(DexExecutionResult::Value(value.clone()))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_and_set,
            Some(&test_invocation(vec![updater, holder, next_value.clone(),])),
        ),
        Some(DexExecutionResult::Value(value))
    );
    assert_eq!(
        program.invoke_builtin_method(&get, Some(&created_invocation)),
        Some(DexExecutionResult::Value(next_value))
    );
}

#[test]
fn builtin_atomic_integer_field_updater_reads_writes_and_compares_int_fields() {
    let program = DexProgram::new(Vec::new());
    let new_updater = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/atomic/AtomicIntegerFieldUpdater;".to_owned(),
        name: "newUpdater".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LLL".to_owned(),
            return_type: "Ljava/util/concurrent/atomic/AtomicIntegerFieldUpdater;".to_owned(),
            parameters: vec![
                "Ljava/lang/Class;".to_owned(),
                "Ljava/lang/String;".to_owned(),
            ],
        },
    };
    let get = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/atomic/AtomicIntegerFieldUpdater;".to_owned(),
        name: "get".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "IL".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let set = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/atomic/AtomicIntegerFieldUpdater;".to_owned(),
        name: "set".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VLI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned(), "I".to_owned()],
        },
    };
    let compare_and_set = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/atomic/AtomicIntegerFieldUpdater;".to_owned(),
        name: "compareAndSet".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZLII".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec![
                "Ljava/lang/Object;".to_owned(),
                "I".to_owned(),
                "I".to_owned(),
            ],
        },
    };
    let increment_and_get = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/atomic/AtomicIntegerFieldUpdater;".to_owned(),
        name: "incrementAndGet".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "IL".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let get_and_increment = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/atomic/AtomicIntegerFieldUpdater;".to_owned(),
        name: "getAndIncrement".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "IL".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let get_and_decrement = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/atomic/AtomicIntegerFieldUpdater;".to_owned(),
        name: "getAndDecrement".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "IL".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let add_and_get = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/atomic/AtomicIntegerFieldUpdater;".to_owned(),
        name: "addAndGet".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ILI".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned(), "I".to_owned()],
        },
    };
    let decrement_and_get = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/atomic/AtomicIntegerFieldUpdater;".to_owned(),
        name: "decrementAndGet".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "IL".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let holder = DexValue::Object(DexObjectRef {
        id: 1,
        class_descriptor: std::rc::Rc::from("Lexample/Holder;"),
    });
    let field = DexFieldId {
        class_descriptor: "Lexample/Holder;".to_owned(),
        type_descriptor: "I".to_owned(),
        name: "state".to_owned(),
    };

    let Some(DexExecutionResult::Value(updater)) = program.invoke_builtin_method(
        &new_updater,
        Some(&test_invocation(vec![
            DexValue::Reference(std::rc::Rc::from("Ljava/lang/Class;:Lexample/Holder;")),
            DexValue::String(std::rc::Rc::from("state")),
        ])),
    ) else {
        panic!("AtomicIntegerFieldUpdater.newUpdater should return an updater reference");
    };

    assert_eq!(
        program.invoke_builtin_method(
            &get,
            Some(&test_invocation(vec![updater.clone(), holder.clone()]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set,
            Some(&test_invocation(vec![
                updater.clone(),
                holder.clone(),
                DexValue::Int(5),
            ])),
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&holder, &field),
        Some(DexValue::Int(5))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &compare_and_set,
            Some(&test_invocation(vec![
                updater.clone(),
                holder.clone(),
                DexValue::Int(4),
                DexValue::Int(7),
            ])),
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &compare_and_set,
            Some(&test_invocation(vec![
                updater.clone(),
                holder.clone(),
                DexValue::Int(5),
                DexValue::Int(7),
            ])),
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &increment_and_get,
            Some(&test_invocation(vec![updater.clone(), holder.clone()]))
        ),
        Some(DexExecutionResult::Int(8))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_and_increment,
            Some(&test_invocation(vec![updater.clone(), holder.clone()]))
        ),
        Some(DexExecutionResult::Int(8))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_and_decrement,
            Some(&test_invocation(vec![updater.clone(), holder.clone()]))
        ),
        Some(DexExecutionResult::Int(9))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &decrement_and_get,
            Some(&test_invocation(vec![updater.clone(), holder.clone()]))
        ),
        Some(DexExecutionResult::Int(7))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &add_and_get,
            Some(&test_invocation(vec![
                updater.clone(),
                holder.clone(),
                DexValue::Int(4),
            ]))
        ),
        Some(DexExecutionResult::Int(11))
    );
    assert_eq!(
        program.invoke_builtin_method(&get, Some(&test_invocation(vec![updater, holder]))),
        Some(DexExecutionResult::Int(11))
    );
}

#[test]
fn builtin_atomic_long_field_updater_reads_writes_and_compares_long_fields() {
    let program = DexProgram::new(Vec::new());
    let new_updater = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/atomic/AtomicLongFieldUpdater;".to_owned(),
        name: "newUpdater".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LLL".to_owned(),
            return_type: "Ljava/util/concurrent/atomic/AtomicLongFieldUpdater;".to_owned(),
            parameters: vec![
                "Ljava/lang/Class;".to_owned(),
                "Ljava/lang/String;".to_owned(),
            ],
        },
    };
    let get = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/atomic/AtomicLongFieldUpdater;".to_owned(),
        name: "get".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "JL".to_owned(),
            return_type: "J".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let get_and_increment = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/atomic/AtomicLongFieldUpdater;".to_owned(),
        name: "getAndIncrement".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "JL".to_owned(),
            return_type: "J".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let increment_and_get = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/atomic/AtomicLongFieldUpdater;".to_owned(),
        name: "incrementAndGet".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "JL".to_owned(),
            return_type: "J".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let add_and_get = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/atomic/AtomicLongFieldUpdater;".to_owned(),
        name: "addAndGet".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "JLJ".to_owned(),
            return_type: "J".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned(), "J".to_owned()],
        },
    };
    let set = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/atomic/AtomicLongFieldUpdater;".to_owned(),
        name: "set".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VLJ".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned(), "J".to_owned()],
        },
    };
    let compare_and_set = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/atomic/AtomicLongFieldUpdater;".to_owned(),
        name: "compareAndSet".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZLJJ".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec![
                "Ljava/lang/Object;".to_owned(),
                "J".to_owned(),
                "J".to_owned(),
            ],
        },
    };
    let holder = DexValue::Object(DexObjectRef {
        id: 1,
        class_descriptor: std::rc::Rc::from("Lexample/Holder;"),
    });

    let Some(DexExecutionResult::Value(updater)) = program.invoke_builtin_method(
        &new_updater,
        Some(&test_invocation(vec![
            DexValue::Reference(std::rc::Rc::from("Ljava/lang/Class;:Lexample/Holder;")),
            DexValue::String(std::rc::Rc::from("counter")),
        ])),
    ) else {
        panic!("AtomicLongFieldUpdater.newUpdater should return an updater reference");
    };

    assert_eq!(
        program.invoke_builtin_method(
            &get,
            Some(&test_invocation(vec![updater.clone(), holder.clone()]))
        ),
        Some(DexExecutionResult::Value(DexValue::Long(0)))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set,
            Some(&test_invocation(vec![
                updater.clone(),
                holder.clone(),
                DexValue::Long(5),
                DexValue::Long(5),
            ])),
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &compare_and_set,
            Some(&test_invocation(vec![
                updater.clone(),
                holder.clone(),
                DexValue::Long(5),
                DexValue::Long(5),
                DexValue::Long(8),
                DexValue::Long(8),
            ])),
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_and_increment,
            Some(&test_invocation(vec![updater.clone(), holder.clone()]))
        ),
        Some(DexExecutionResult::Value(DexValue::Long(8)))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get,
            Some(&test_invocation(vec![updater.clone(), holder.clone()]))
        ),
        Some(DexExecutionResult::Value(DexValue::Long(9)))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &increment_and_get,
            Some(&test_invocation(vec![updater.clone(), holder.clone()]))
        ),
        Some(DexExecutionResult::Value(DexValue::Long(10)))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &add_and_get,
            Some(&test_invocation(vec![
                updater.clone(),
                holder.clone(),
                DexValue::Long(4),
                DexValue::Long(4),
            ]))
        ),
        Some(DexExecutionResult::Value(DexValue::Long(14)))
    );
    assert_eq!(
        program.invoke_builtin_method(&get, Some(&test_invocation(vec![updater, holder]))),
        Some(DexExecutionResult::Value(DexValue::Long(14)))
    );
}

#[test]
fn builtin_unsafe_object_field_offset_drives_compare_and_swap_object() {
    let program = DexProgram::new(Vec::new());
    let object_field_offset = DexMethodId {
        class_descriptor: "Lsun/misc/Unsafe;".to_owned(),
        name: "objectFieldOffset".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "JL".to_owned(),
            return_type: "J".to_owned(),
            parameters: vec!["Ljava/lang/reflect/Field;".to_owned()],
        },
    };
    let compare_and_swap_object = DexMethodId {
        class_descriptor: "Lsun/misc/Unsafe;".to_owned(),
        name: "compareAndSwapObject".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZLJLL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec![
                "Ljava/lang/Object;".to_owned(),
                "J".to_owned(),
                "Ljava/lang/Object;".to_owned(),
                "Ljava/lang/Object;".to_owned(),
            ],
        },
    };
    let put_object_volatile = DexMethodId {
        class_descriptor: "Lsun/misc/Unsafe;".to_owned(),
        name: "putObjectVolatile".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VLJL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "Ljava/lang/Object;".to_owned(),
                "J".to_owned(),
                "Ljava/lang/Object;".to_owned(),
            ],
        },
    };
    let get_object_volatile = DexMethodId {
        class_descriptor: "Lsun/misc/Unsafe;".to_owned(),
        name: "getObjectVolatile".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LLJ".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned(), "J".to_owned()],
        },
    };
    let get_and_set_object = DexMethodId {
        class_descriptor: "Lsun/misc/Unsafe;".to_owned(),
        name: "getAndSetObject".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LLJL".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: vec![
                "Ljava/lang/Object;".to_owned(),
                "J".to_owned(),
                "Ljava/lang/Object;".to_owned(),
            ],
        },
    };
    let unsafe_value = DexValue::Reference(std::rc::Rc::from("Lsun/misc/Unsafe;:theUnsafe"));
    let field = DexFieldId {
        class_descriptor: "Lexample/Holder;".to_owned(),
        type_descriptor: "Ljava/lang/Object;".to_owned(),
        name: "_state".to_owned(),
    };
    let field_reference = DexValue::Reference(std::rc::Rc::from(
        "Ljava/lang/reflect/Field;:Lexample/Holder;:_state:Ljava/lang/Object;:0",
    ));
    let holder = DexValue::Object(DexObjectRef {
        id: 1,
        class_descriptor: std::rc::Rc::from("Lexample/Holder;"),
    });
    let next = DexValue::Reference(std::rc::Rc::from("Lexample/State;.ready"));
    let later = DexValue::Reference(std::rc::Rc::from("Lexample/State;.later"));
    let final_value = DexValue::Reference(std::rc::Rc::from("Lexample/State;.final"));

    let Some(DexExecutionResult::Value(DexValue::Long(offset))) = program.invoke_builtin_method(
        &object_field_offset,
        Some(&test_invocation(vec![
            unsafe_value.clone(),
            field_reference,
        ])),
    ) else {
        panic!("objectFieldOffset should return a long offset");
    };

    assert_eq!(
        program.invoke_builtin_method(
            &compare_and_swap_object,
            Some(&test_invocation(vec![
                unsafe_value.clone(),
                holder.clone(),
                DexValue::Long(offset),
                DexValue::Long(offset),
                DexValue::Null,
                next.clone(),
            ])),
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.read_stored_instance_field(&holder, &field),
        Some(next)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &put_object_volatile,
            Some(&test_invocation(vec![
                unsafe_value.clone(),
                holder.clone(),
                DexValue::Long(offset),
                DexValue::Long(offset),
                later.clone(),
            ])),
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&holder, &field),
        Some(later.clone())
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_and_set_object,
            Some(&test_invocation(vec![
                unsafe_value.clone(),
                holder.clone(),
                DexValue::Long(offset),
                DexValue::Long(offset),
                final_value.clone(),
            ])),
        ),
        Some(DexExecutionResult::Value(later))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_object_volatile,
            Some(&test_invocation(vec![
                unsafe_value,
                holder,
                DexValue::Long(offset),
                DexValue::Long(offset),
            ])),
        ),
        Some(DexExecutionResult::Value(final_value))
    );
}

#[test]
fn builtin_weak_reference_get_returns_lifecycle_owner_reference() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Ljava/lang/ref/WeakReference;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let weak_reference_get = DexMethodId {
        class_descriptor: "Ljava/lang/ref/WeakReference;".to_owned(),
        name: "get".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let reference_get = DexMethodId {
        class_descriptor: "Ljava/lang/ref/Reference;".to_owned(),
        name: "get".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let weak_reference = DexValue::Reference(std::rc::Rc::from("weak-reference"));
    let referent = DexValue::Reference(std::rc::Rc::from("referent"));
    let constructor_invocation = test_invocation(vec![weak_reference.clone(), referent.clone()]);
    let get_invocation = test_invocation(vec![weak_reference]);

    assert_eq!(
        program.invoke_builtin_method(&weak_reference_get, None),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Landroidx/lifecycle/LifecycleOwner;")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&reference_get, None),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Landroidx/lifecycle/LifecycleOwner;")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&constructor, Some(&constructor_invocation)),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&weak_reference_get, Some(&get_invocation)),
        Some(DexExecutionResult::Value(referent))
    );
}

#[test]
fn builtin_reference_clear_makes_a_subsequent_get_return_null() {
    // cite: libcore Reference.java#clear
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Ljava/lang/ref/WeakReference;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let clear = DexMethodId {
        class_descriptor: "Ljava/lang/ref/WeakReference;".to_owned(),
        name: "clear".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let weak_reference_get = DexMethodId {
        class_descriptor: "Ljava/lang/ref/WeakReference;".to_owned(),
        name: "get".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let weak_reference = DexValue::Reference(std::rc::Rc::from("clearable-weak-reference"));
    let referent = DexValue::Reference(std::rc::Rc::from("referent"));

    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![weak_reference.clone(), referent]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&clear, Some(&test_invocation(vec![weak_reference.clone()]))),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &weak_reference_get,
            Some(&test_invocation(vec![weak_reference]))
        ),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
}

#[test]
fn builtin_reference_queue_and_queued_weak_reference_are_supported() {
    let program = DexProgram::new(Vec::new());
    let queue_constructor = DexMethodId {
        class_descriptor: "Ljava/lang/ref/ReferenceQueue;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let poll = DexMethodId {
        class_descriptor: "Ljava/lang/ref/ReferenceQueue;".to_owned(),
        name: "poll".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/ref/Reference;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let weak_reference_constructor = DexMethodId {
        class_descriptor: "Ljava/lang/ref/WeakReference;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VLL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "Ljava/lang/Object;".to_owned(),
                "Ljava/lang/ref/ReferenceQueue;".to_owned(),
            ],
        },
    };
    let weak_reference_get = DexMethodId {
        class_descriptor: "Ljava/lang/ref/WeakReference;".to_owned(),
        name: "get".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let queue = DexValue::Reference(std::rc::Rc::from("reference-queue"));
    let weak_reference = DexValue::Reference(std::rc::Rc::from("queued-weak-reference"));
    let referent = DexValue::Reference(std::rc::Rc::from("referent"));

    assert_eq!(
        program.invoke_builtin_method(
            &queue_constructor,
            Some(&test_invocation(vec![queue.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&poll, Some(&test_invocation(vec![queue.clone()]))),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &weak_reference_constructor,
            Some(&test_invocation(vec![
                weak_reference.clone(),
                referent.clone(),
                queue,
            ])),
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &weak_reference_get,
            Some(&test_invocation(vec![weak_reference]))
        ),
        Some(DexExecutionResult::Value(referent))
    );
}

#[test]
fn libcore_count_down_latch_constructor_stores_count_and_rejects_negative() {
    // cite: libcore CountDownLatch.java#CountDownLatch, Sync
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/CountDownLatch;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let latch = DexValue::Object(DexObjectRef {
        id: 600,
        class_descriptor: std::rc::Rc::from("Ljava/util/concurrent/CountDownLatch;"),
    });
    let rejected = DexValue::Object(DexObjectRef {
        id: 601,
        class_descriptor: std::rc::Rc::from("Ljava/util/concurrent/CountDownLatch;"),
    });

    for count in [3, 0] {
        assert_eq!(
            program.invoke_builtin_method(
                &constructor,
                Some(&test_invocation(vec![latch.clone(), DexValue::Int(count)]))
            ),
            Some(DexExecutionResult::Void)
        );
        assert_eq!(
            program
                .runtime
                .borrow()
                .atomic_integer_values
                .get(&map_identity_key(&latch))
                .copied(),
            Some(count)
        );
    }

    let Some(DexExecutionResult::Throw(exception)) = program.invoke_builtin_method(
        &constructor,
        Some(&test_invocation(vec![rejected.clone(), DexValue::Int(-1)])),
    ) else {
        panic!("negative count should throw");
    };
    assert!(program.value_matches_type(&exception, "Ljava/lang/IllegalArgumentException;"));
    assert!(!program
        .runtime
        .borrow()
        .atomic_integer_values
        .contains_key(&map_identity_key(&rejected)));
}

#[test]
fn libcore_count_down_latch_count_down_decrements_to_zero_and_stops() {
    // cite: libcore CountDownLatch.java#countDown, tryReleaseShared
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/CountDownLatch;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let count_down = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/CountDownLatch;".to_owned(),
        name: "countDown".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let latch = program.allocate_object("Ljava/util/concurrent/CountDownLatch;".to_owned());
    let unconstructed = program.allocate_object("Ljava/util/concurrent/CountDownLatch;".to_owned());

    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![latch.clone(), DexValue::Int(2)]))
        ),
        Some(DexExecutionResult::Void)
    );
    for expected in [1, 0, 0] {
        assert_eq!(
            program.invoke_builtin_method(&count_down, Some(&test_invocation(vec![latch.clone()]))),
            Some(DexExecutionResult::Void)
        );
        assert_eq!(
            program
                .runtime
                .borrow()
                .atomic_integer_values
                .get(&map_identity_key(&latch))
                .copied(),
            Some(expected)
        );
    }
    assert_eq!(
        program.invoke_builtin_method(&count_down, Some(&test_invocation(vec![unconstructed]))),
        None
    );
}

const THREAD_POOL_EXECUTOR: &str = "Ljava/util/concurrent/ThreadPoolExecutor;";

fn thread_pool_executor_constructor() -> DexMethodId {
    DexMethodId {
        class_descriptor: THREAD_POOL_EXECUTOR.to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VIIJLLL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "I".to_owned(),
                "I".to_owned(),
                "J".to_owned(),
                "Ljava/util/concurrent/TimeUnit;".to_owned(),
                "Ljava/util/concurrent/BlockingQueue;".to_owned(),
                "Ljava/util/concurrent/ThreadFactory;".to_owned(),
            ],
        },
    }
}

fn thread_pool_executor_arguments(
    executor: &DexValue,
    core: i32,
    maximum: i32,
    keep_alive: i64,
    unit: DexValue,
    queue: DexValue,
    factory: DexValue,
) -> DexInvocation {
    test_invocation(vec![
        executor.clone(),
        DexValue::Int(core),
        DexValue::Int(maximum),
        DexValue::Long(keep_alive),
        DexValue::Long(keep_alive),
        unit,
        queue,
        factory,
    ])
}

fn seconds_unit() -> DexValue {
    DexValue::Reference(std::rc::Rc::from("Ljava/util/concurrent/TimeUnit;.SECONDS"))
}

fn thread_pool_executor_dependency(descriptor: &str) -> DexValue {
    DexValue::Reference(std::rc::Rc::from(descriptor))
}

fn construct_thread_pool_executor(
    program: &DexProgram,
    core: i32,
    maximum: i32,
    keep_alive: i64,
) -> DexValue {
    let executor = program.allocate_object(THREAD_POOL_EXECUTOR.to_owned());
    let invocation = thread_pool_executor_arguments(
        &executor,
        core,
        maximum,
        keep_alive,
        seconds_unit(),
        thread_pool_executor_dependency("Ljava/util/concurrent/LinkedBlockingQueue;:queue"),
        thread_pool_executor_dependency("Ljava/util/concurrent/ThreadFactory;:factory"),
    );

    assert_eq!(
        program.invoke_builtin_method(&thread_pool_executor_constructor(), Some(&invocation)),
        Some(DexExecutionResult::Void)
    );

    executor
}

fn thread_pool_executor_field(name: &str, type_descriptor: &str) -> DexFieldId {
    DexFieldId {
        class_descriptor: THREAD_POOL_EXECUTOR.to_owned(),
        type_descriptor: type_descriptor.to_owned(),
        name: name.to_owned(),
    }
}

fn allow_core_thread_time_out_method() -> DexMethodId {
    DexMethodId {
        class_descriptor: THREAD_POOL_EXECUTOR.to_owned(),
        name: "allowCoreThreadTimeOut".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VZ".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Z".to_owned()],
        },
    }
}

#[test]
fn libcore_thread_pool_executor_constructor_stores_pool_state_in_nanoseconds() {
    // cite: libcore ThreadPoolExecutor.java#ThreadPoolExecutor
    let program = DexProgram::new(Vec::new());
    let executor = construct_thread_pool_executor(&program, 2, 4, 60);

    assert_eq!(
        program.read_stored_instance_field(
            &executor,
            &thread_pool_executor_field("corePoolSize", "I")
        ),
        Some(DexValue::Int(2))
    );
    assert_eq!(
        program.read_stored_instance_field(
            &executor,
            &thread_pool_executor_field("maximumPoolSize", "I")
        ),
        Some(DexValue::Int(4))
    );
    assert_eq!(
        program.read_stored_instance_field(
            &executor,
            &thread_pool_executor_field("keepAliveTime", "J")
        ),
        Some(DexValue::Long(60_000_000_000))
    );
    assert_eq!(
        program.read_stored_instance_field(
            &executor,
            &thread_pool_executor_field("workQueue", "Ljava/util/concurrent/BlockingQueue;")
        ),
        Some(thread_pool_executor_dependency(
            "Ljava/util/concurrent/LinkedBlockingQueue;:queue"
        ))
    );
}

fn thread_pool_executor_with_handler_constructor() -> DexMethodId {
    let mut method = thread_pool_executor_constructor();
    method.proto.shorty = "VIIJLLLL".to_owned();
    method
        .proto
        .parameters
        .push("Ljava/util/concurrent/RejectedExecutionHandler;".to_owned());
    method
}

#[test]
fn libcore_thread_pool_executor_seven_argument_constructor_stores_and_validates_handler() {
    // cite: libcore ThreadPoolExecutor.java#ThreadPoolExecutor, handler
    let program = DexProgram::new(Vec::new());
    let queue = thread_pool_executor_dependency("Ljava/util/concurrent/LinkedBlockingQueue;:queue");
    let factory = thread_pool_executor_dependency("Ljava/util/concurrent/ThreadFactory;:factory");
    let handler = thread_pool_executor_dependency(
        "Ljava/util/concurrent/ThreadPoolExecutor$AbortPolicy;:handler",
    );
    let construct = |core: i32, maximum: i32, handler: DexValue| {
        let executor = program.allocate_object(THREAD_POOL_EXECUTOR.to_owned());
        let mut arguments = thread_pool_executor_arguments(
            &executor,
            core,
            maximum,
            60,
            seconds_unit(),
            queue.clone(),
            factory.clone(),
        )
        .arguments
        .to_vec();
        arguments.push(handler);
        let result = program.invoke_builtin_method(
            &thread_pool_executor_with_handler_constructor(),
            Some(&test_invocation(arguments)),
        );

        (executor, result)
    };

    let (executor, result) = construct(2, 4, handler.clone());

    assert_eq!(result, Some(DexExecutionResult::Void));
    assert_eq!(
        program.read_stored_instance_field(
            &executor,
            &thread_pool_executor_field(
                "handler",
                "Ljava/util/concurrent/RejectedExecutionHandler;"
            )
        ),
        Some(handler.clone())
    );
    assert_eq!(
        program.read_stored_instance_field(
            &executor,
            &thread_pool_executor_field("keepAliveTime", "J")
        ),
        Some(DexValue::Long(60_000_000_000))
    );

    let (_, null_handler) = construct(2, 4, DexValue::Null);
    let Some(DexExecutionResult::Throw(exception)) = null_handler else {
        panic!("null handler should throw NullPointerException");
    };
    assert!(program.value_matches_type(&exception, "Ljava/lang/NullPointerException;"));

    let (_, invalid) = construct(-1, 4, DexValue::Null);
    let Some(DexExecutionResult::Throw(exception)) = invalid else {
        panic!("invalid core size should throw IllegalArgumentException");
    };
    assert!(program.value_matches_type(&exception, "Ljava/lang/IllegalArgumentException;"));
}

#[test]
fn libcore_thread_pool_executor_abort_policy_constructor_returns_void() {
    // cite: libcore ThreadPoolExecutor.java#AbortPolicy
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Ljava/util/concurrent/ThreadPoolExecutor$AbortPolicy;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let policy =
        program.allocate_object("Ljava/util/concurrent/ThreadPoolExecutor$AbortPolicy;".to_owned());

    assert_eq!(
        program.invoke_builtin_method(&constructor, Some(&test_invocation(vec![policy]))),
        Some(DexExecutionResult::Void)
    );
}

#[test]
fn libcore_thread_pool_executor_discard_oldest_policy_constructor_returns_void() {
    // cite: libcore ThreadPoolExecutor.java#DiscardOldestPolicy
    let program = DexProgram::new(Vec::new());
    let constructor = builtin_method(
        "Ljava/util/concurrent/ThreadPoolExecutor$DiscardOldestPolicy;",
        "<init>",
        "V",
        &[],
    );
    let policy = program.allocate_object(
        "Ljava/util/concurrent/ThreadPoolExecutor$DiscardOldestPolicy;".to_owned(),
    );

    assert_eq!(
        program.invoke_builtin_method(&constructor, Some(&test_invocation(vec![policy]))),
        Some(DexExecutionResult::Void)
    );
}

#[test]
fn libcore_thread_pool_executor_constructor_rejects_invalid_arguments_in_source_order() {
    // cite: libcore ThreadPoolExecutor.java#ThreadPoolExecutor
    let program = DexProgram::new(Vec::new());
    let queue = thread_pool_executor_dependency("Ljava/util/concurrent/LinkedBlockingQueue;:queue");
    let factory = thread_pool_executor_dependency("Ljava/util/concurrent/ThreadFactory;:factory");
    let throws = |core: i32,
                  maximum: i32,
                  keep_alive: i64,
                  unit: DexValue,
                  queue: DexValue,
                  factory: DexValue,
                  expected: &str| {
        let executor = program.allocate_object(THREAD_POOL_EXECUTOR.to_owned());
        let invocation = thread_pool_executor_arguments(
            &executor, core, maximum, keep_alive, unit, queue, factory,
        );
        let Some(DexExecutionResult::Throw(exception)) =
            program.invoke_builtin_method(&thread_pool_executor_constructor(), Some(&invocation))
        else {
            panic!("constructor should throw {expected}");
        };

        assert!(
            program.value_matches_type(&exception, expected),
            "{expected}"
        );
    };
    let illegal = "Ljava/lang/IllegalArgumentException;";
    let null_pointer = "Ljava/lang/NullPointerException;";

    throws(
        -1,
        4,
        1,
        seconds_unit(),
        queue.clone(),
        factory.clone(),
        illegal,
    );
    throws(
        1,
        0,
        1,
        seconds_unit(),
        queue.clone(),
        factory.clone(),
        illegal,
    );
    throws(
        5,
        4,
        1,
        seconds_unit(),
        queue.clone(),
        factory.clone(),
        illegal,
    );
    throws(
        1,
        4,
        -1,
        seconds_unit(),
        queue.clone(),
        factory.clone(),
        illegal,
    );
    throws(
        1,
        4,
        1,
        seconds_unit(),
        DexValue::Null,
        factory.clone(),
        null_pointer,
    );
    throws(
        1,
        4,
        1,
        seconds_unit(),
        queue.clone(),
        DexValue::Null,
        null_pointer,
    );
    throws(
        1,
        4,
        1,
        DexValue::Null,
        queue.clone(),
        factory.clone(),
        null_pointer,
    );
    throws(-1, 4, 1, seconds_unit(), DexValue::Null, factory, illegal);
}

#[test]
fn libcore_thread_pool_executor_execute_posts_runnable_and_rejects_null() {
    // cite: libcore ThreadPoolExecutor.java#execute
    let program = DexProgram::new(Vec::new());
    let executor = construct_thread_pool_executor(&program, 1, 1, 0);
    let execute = DexMethodId {
        class_descriptor: THREAD_POOL_EXECUTOR.to_owned(),
        name: "execute".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/lang/Runnable;".to_owned()],
        },
    };
    let runnable = program.allocate_object("Lcom/example/Task;".to_owned());

    assert_eq!(
        program.invoke_builtin_method(
            &execute,
            Some(&test_invocation(vec![executor.clone(), runnable]))
        ),
        Some(DexExecutionResult::Void)
    );
    let Some(DexExecutionResult::Throw(exception)) = program.invoke_builtin_method(
        &execute,
        Some(&test_invocation(vec![executor, DexValue::Null])),
    ) else {
        panic!("execute(null) should throw");
    };
    assert!(program.value_matches_type(&exception, "Ljava/lang/NullPointerException;"));
}

#[test]
fn libcore_thread_pool_executor_allow_core_thread_time_out_requires_a_nonzero_keep_alive() {
    // cite: libcore ThreadPoolExecutor.java#allowCoreThreadTimeOut
    let program = DexProgram::new(Vec::new());
    let method = allow_core_thread_time_out_method();
    let flag = thread_pool_executor_field("allowCoreThreadTimeOut", "Z");
    let permanent = construct_thread_pool_executor(&program, 1, 1, 0);
    let expiring = construct_thread_pool_executor(&program, 1, 1, 5);

    let Some(DexExecutionResult::Throw(exception)) = program.invoke_builtin_method(
        &method,
        Some(&test_invocation(vec![permanent.clone(), DexValue::Int(1)])),
    ) else {
        panic!("zero keep alive should reject allowCoreThreadTimeOut(true)");
    };
    assert!(program.value_matches_type(&exception, "Ljava/lang/IllegalArgumentException;"));
    assert_eq!(
        program.read_stored_instance_field(
            &exception,
            &DexFieldId {
                class_descriptor: "Ljava/lang/Throwable;".to_owned(),
                type_descriptor: "Ljava/lang/String;".to_owned(),
                name: "detailMessage".to_owned(),
            }
        ),
        Some(DexValue::String(std::rc::Rc::from(
            "Core threads must have nonzero keep alive times"
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![permanent.clone(), DexValue::Int(0)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![expiring.clone(), DexValue::Int(1)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&expiring, &flag),
        Some(DexValue::Int(1))
    );
    assert_eq!(program.read_stored_instance_field(&permanent, &flag), None);
}

#[test]
fn libcore_executors_pools_derive_keep_alive_for_allow_core_thread_time_out() {
    // cite: libcore Executors.java#newFixedThreadPool, newCachedThreadPool
    let program = DexProgram::new(Vec::new());
    let method = allow_core_thread_time_out_method();
    let call = |reference: &str| {
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::Reference(std::rc::Rc::from(reference)),
                DexValue::Int(1),
            ])),
        )
    };

    assert!(matches!(
        call("Ljava/util/concurrent/ExecutorService;.fixed/4"),
        Some(DexExecutionResult::Throw(_))
    ));
    assert_eq!(
        call("Ljava/util/concurrent/ExecutorService;.cached"),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(call("Ljava/util/concurrent/ExecutorService;.single"), None);
}

#[test]
fn libcore_scheduled_thread_pool_executor_constructor_stores_default_pool_state() {
    // cite: libcore ScheduledThreadPoolExecutor.java#ScheduledThreadPoolExecutor, DEFAULT_KEEPALIVE_MILLIS
    let program = DexProgram::new(Vec::new());
    let constructor = builtin_method(
        "Ljava/util/concurrent/ScheduledThreadPoolExecutor;",
        "<init>",
        "V",
        &["I", "Ljava/util/concurrent/ThreadFactory;"],
    );
    let factory = thread_pool_executor_dependency("Ljava/util/concurrent/ThreadFactory;:factory");
    let executor =
        program.allocate_object("Ljava/util/concurrent/ScheduledThreadPoolExecutor;".to_owned());

    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![
                executor.clone(),
                DexValue::Int(2),
                factory.clone()
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    let field = |name, type_descriptor| {
        program.read_stored_instance_field(
            &executor,
            &thread_pool_executor_field(name, type_descriptor),
        )
    };
    assert_eq!(field("corePoolSize", "I"), Some(DexValue::Int(2)));
    assert_eq!(field("maximumPoolSize", "I"), Some(DexValue::Int(i32::MAX)));
    assert_eq!(
        field("keepAliveTime", "J"),
        Some(DexValue::Long(10_000_000))
    );
    assert_eq!(
        field("threadFactory", "Ljava/util/concurrent/ThreadFactory;"),
        Some(factory)
    );
    let queue = field("workQueue", "Ljava/util/concurrent/BlockingQueue;")
        .expect("work queue should be stored");
    assert_eq!(
        value_class_descriptor(&queue),
        Some("Ljava/util/concurrent/ScheduledThreadPoolExecutor$DelayedWorkQueue;")
    );
    assert!(program.is_thread_pool_executor(&executor));
    assert!(
        program.value_matches_type(&executor, "Ljava/util/concurrent/ScheduledExecutorService;")
    );
    assert!(program.value_matches_type(&executor, "Ljava/util/concurrent/Executor;"));
}

// cite: libcore ExecutorService.java#shutdown, isShutdown, isTerminated, awaitTermination
#[test]
fn executor_shutdown_state_is_observable() {
    let program = DexProgram::new(Vec::new());
    let executor = DexValue::Reference(std::rc::Rc::from(
        "Ljava/util/concurrent/ScheduledExecutorService;.single",
    ));
    let method = |name: &str, proto: crate::dex::DexProto| DexMethodId {
        class_descriptor: "Ljava/util/concurrent/ScheduledExecutorService;".to_owned(),
        name: name.to_owned(),
        proto,
    };
    let shutdown = method(
        "shutdown",
        crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    );
    let is_shutdown = method(
        "isShutdown",
        crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    );
    let is_terminated = method(
        "isTerminated",
        crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    );
    let await_termination = method(
        "awaitTermination",
        crate::dex::DexProto {
            shorty: "ZJL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["J".to_owned(), "Ljava/util/concurrent/TimeUnit;".to_owned()],
        },
    );

    assert_eq!(
        program.invoke_builtin_method(&shutdown, Some(&test_invocation(vec![executor.clone()]))),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&is_shutdown, Some(&test_invocation(vec![executor.clone()]))),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &is_terminated,
            Some(&test_invocation(vec![executor.clone()]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &await_termination,
            Some(&test_invocation(vec![
                executor,
                DexValue::Long(0),
                DexValue::Reference(std::rc::Rc::from(
                    "Ljava/util/concurrent/TimeUnit;.MILLISECONDS"
                ))
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
}

#[test]
fn libcore_scheduled_thread_pool_executor_constructor_checks_core_size_before_factory() {
    // cite: libcore ThreadPoolExecutor.java#ThreadPoolExecutor
    let program = DexProgram::new(Vec::new());
    let constructor = builtin_method(
        "Ljava/util/concurrent/ScheduledThreadPoolExecutor;",
        "<init>",
        "V",
        &["I", "Ljava/util/concurrent/ThreadFactory;"],
    );
    let factory = thread_pool_executor_dependency("Ljava/util/concurrent/ThreadFactory;:factory");
    let construct = |core: i32, factory: DexValue| {
        let executor = program
            .allocate_object("Ljava/util/concurrent/ScheduledThreadPoolExecutor;".to_owned());
        thrown_exception(program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![
                executor,
                DexValue::Int(core),
                factory,
            ])),
        ))
    };

    assert!(program.value_matches_type(
        &construct(-1, factory),
        "Ljava/lang/IllegalArgumentException;"
    ));
    assert!(program.value_matches_type(
        &construct(1, DexValue::Null),
        "Ljava/lang/NullPointerException;"
    ));
    assert!(program.value_matches_type(
        &construct(-1, DexValue::Null),
        "Ljava/lang/IllegalArgumentException;"
    ));
}

#[test]
fn libcore_abstract_executor_service_constructor_is_a_no_op_on_an_abstract_class() {
    // cite: libcore AbstractExecutorService.java#AbstractExecutorService
    let program = DexProgram::new(Vec::new());
    let constructor = builtin_method(
        "Ljava/util/concurrent/AbstractExecutorService;",
        "<init>",
        "V",
        &[],
    );
    let receiver =
        program.allocate_object("Ljava/util/concurrent/AbstractExecutorService;".to_owned());

    assert_eq!(
        program.invoke_builtin_method(&constructor, Some(&test_invocation(vec![receiver.clone()]))),
        Some(DexExecutionResult::Void)
    );
    assert!(program.value_matches_type(&receiver, "Ljava/util/concurrent/ExecutorService;"));
    assert!(is_bootstrap_class_descriptor(
        "Ljava/util/concurrent/AbstractExecutorService;"
    ));
    assert_eq!(
        bootstrap_new_instance_kind("Ljava/util/concurrent/AbstractExecutorService;"),
        Some(BootstrapNewInstanceKind::Abstract)
    );
}

#[test]
fn libcore_thread_pool_executor_set_keep_alive_time_rewrites_nanos_and_guards_core_time_out() {
    // cite: libcore ThreadPoolExecutor.java#setKeepAliveTime
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: THREAD_POOL_EXECUTOR.to_owned(),
        name: "setKeepAliveTime".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VJL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["J".to_owned(), "Ljava/util/concurrent/TimeUnit;".to_owned()],
        },
    };
    let keep_alive = thread_pool_executor_field("keepAliveTime", "J");
    let executor = construct_thread_pool_executor(&program, 1, 2, 5);
    let call = |time: i64| {
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                executor.clone(),
                DexValue::Long(time),
                DexValue::Long(time),
                seconds_unit(),
            ])),
        )
    };

    assert_eq!(call(7), Some(DexExecutionResult::Void));
    assert_eq!(
        program.read_stored_instance_field(&executor, &keep_alive),
        Some(DexValue::Long(7_000_000_000))
    );
    let Some(DexExecutionResult::Throw(exception)) = call(-1) else {
        panic!("negative keep alive should throw");
    };
    assert!(program.value_matches_type(&exception, "Ljava/lang/IllegalArgumentException;"));
    assert_eq!(
        program.invoke_builtin_method(
            &allow_core_thread_time_out_method(),
            Some(&test_invocation(vec![executor.clone(), DexValue::Int(1)]))
        ),
        Some(DexExecutionResult::Void)
    );
    let Some(DexExecutionResult::Throw(exception)) = call(0) else {
        panic!("zero keep alive with core time out should throw");
    };
    assert!(program.value_matches_type(&exception, "Ljava/lang/IllegalArgumentException;"));
    assert_eq!(
        program.read_stored_instance_field(&executor, &keep_alive),
        Some(DexValue::Long(7_000_000_000))
    );
}

#[test]
fn libcore_scheduled_thread_pool_executor_submit_schedules_runnable_immediately() {
    // cite: libcore ScheduledThreadPoolExecutor.java#submit, schedule
    let program = DexProgram::new(Vec::new());
    let executor =
        program.allocate_object("Ljava/util/concurrent/ScheduledThreadPoolExecutor;".to_owned());
    let command = program.allocate_object("Lcom/example/Task;".to_owned());
    let method = builtin_method(
        "Ljava/util/concurrent/ScheduledThreadPoolExecutor;",
        "submit",
        "Ljava/util/concurrent/Future;",
        &["Ljava/lang/Runnable;"],
    );
    let result = program.invoke_builtin_method(
        &method,
        Some(&test_invocation(vec![executor.clone(), command.clone()])),
    );
    let Some(DexExecutionResult::Value(task)) = result else {
        panic!("submit should return a future");
    };
    assert_eq!(
        program.read_stored_instance_field(&task, &future_task_runnable_field()),
        Some(command)
    );
    let posts = program.runtime.borrow().handler_posts.clone();
    assert!(posts
        .iter()
        .any(|post| post.runnable == task && post.delay_millis == 0));
    for null in [DexValue::Null, DexValue::Int(0)] {
        let result = program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![executor.clone(), null])),
        );
        assert!(program.value_matches_type(
            &thrown_exception(result),
            "Ljava/lang/NullPointerException;"
        ));
    }
}

// cite: libcore AbstractExecutorService.java#submit, newTaskFor
#[test]
fn libcore_thread_pool_executor_submit_callable_returns_future_task() {
    let program = DexProgram::new(Vec::new());
    let executor = program.allocate_object("Ljava/util/concurrent/ThreadPoolExecutor;".to_owned());
    let callable = program.allocate_object("Lcom/example/Callable;".to_owned());
    let method = builtin_method(
        "Ljava/util/concurrent/ThreadPoolExecutor;",
        "submit",
        "Ljava/util/concurrent/Future;",
        &["Ljava/util/concurrent/Callable;"],
    );
    let result = program.invoke_builtin_method(
        &method,
        Some(&test_invocation(vec![executor, callable.clone()])),
    );
    let Some(DexExecutionResult::Value(task)) = result else {
        panic!("submit should return a future");
    };
    assert_eq!(
        program.read_stored_instance_field(&task, &future_task_callable_field()),
        Some(callable)
    );
    assert!(program
        .runtime
        .borrow()
        .handler_posts
        .iter()
        .any(|post| post.runnable == task && post.delay_millis == 0));
}

#[test]
fn libcore_scheduled_thread_pool_executor_schedule_posts_a_delayed_future_task() {
    // cite: libcore ScheduledThreadPoolExecutor.java#schedule, triggerTime
    let program = DexProgram::new(Vec::new());
    let constructor = builtin_method(
        "Ljava/util/concurrent/ScheduledThreadPoolExecutor;",
        "<init>",
        "V",
        &["I", "Ljava/util/concurrent/ThreadFactory;"],
    );
    let schedule = builtin_method(
        "Ljava/util/concurrent/ScheduledThreadPoolExecutor;",
        "schedule",
        "Ljava/util/concurrent/ScheduledFuture;",
        &[
            "Ljava/lang/Runnable;",
            "J",
            "Ljava/util/concurrent/TimeUnit;",
        ],
    );
    let executor =
        program.allocate_object("Ljava/util/concurrent/ScheduledThreadPoolExecutor;".to_owned());
    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![
                executor.clone(),
                DexValue::Int(1),
                thread_pool_executor_dependency("Ljava/util/concurrent/ThreadFactory;:factory")
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    let command = program.allocate_object("Lcom/example/Task;".to_owned());
    let call = |command: DexValue, delay: i64, unit: DexValue| {
        program.invoke_builtin_method(
            &schedule,
            Some(&test_invocation(vec![
                executor.clone(),
                command,
                DexValue::Long(delay),
                DexValue::Long(delay),
                unit,
            ])),
        )
    };

    let Some(DexExecutionResult::Value(task)) = call(command.clone(), 3, seconds_unit()) else {
        panic!("schedule should return the task");
    };
    assert!(program.value_matches_type(&task, "Ljava/util/concurrent/ScheduledFuture;"));
    assert!(program.value_matches_type(&task, "Ljava/util/concurrent/FutureTask;"));
    let posts = program.runtime.borrow().handler_posts.clone();
    let [post] = posts.as_slice() else {
        panic!("one post expected");
    };
    assert_eq!(post.runnable, task);
    assert_eq!(post.delay_millis, 3000);
    assert_eq!(
        program.read_stored_instance_field(&task, &future_task_runnable_field()),
        Some(command.clone())
    );

    let Some(DexExecutionResult::Value(immediate)) = call(command.clone(), -5, seconds_unit())
    else {
        panic!("negative delay runs as zero");
    };
    assert_eq!(
        program
            .runtime
            .borrow()
            .handler_posts
            .iter()
            .find(|post| post.runnable == immediate)
            .map(|post| post.delay_millis),
        Some(0)
    );
    let nanoseconds = DexValue::Reference(std::rc::Rc::from(
        "Ljava/util/concurrent/TimeUnit;.NANOSECONDS",
    ));
    let Some(DexExecutionResult::Value(sub_millisecond)) = call(command.clone(), 1, nanoseconds)
    else {
        panic!("sub millisecond delay should schedule");
    };
    assert_eq!(
        program
            .runtime
            .borrow()
            .handler_posts
            .iter()
            .find(|post| post.runnable == sub_millisecond)
            .map(|post| post.delay_millis),
        Some(1)
    );
    for result in [
        call(DexValue::Null, 1, seconds_unit()),
        call(command, 1, DexValue::Null),
    ] {
        assert!(program.value_matches_type(
            &thrown_exception(result),
            "Ljava/lang/NullPointerException;"
        ));
    }
}

#[test]
fn builtin_atomic_long_constructor_and_get_and_add() {
    // cite: libcore AtomicLong.java#AtomicLong, getAndAdd
    let program = DexProgram::new(Vec::new());
    let constructor = builtin_method(
        "Ljava/util/concurrent/atomic/AtomicLong;",
        "<init>",
        "V",
        &["J"],
    );
    let get_and_add = builtin_method(
        "Ljava/util/concurrent/atomic/AtomicLong;",
        "getAndAdd",
        "J",
        &["J"],
    );
    let value = program.allocate_object("Ljava/util/concurrent/atomic/AtomicLong;".to_owned());

    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![
                value.clone(),
                DexValue::Long(10),
                DexValue::Long(10)
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_and_add,
            Some(&test_invocation(vec![
                value.clone(),
                DexValue::Long(5),
                DexValue::Long(5)
            ]))
        ),
        Some(DexExecutionResult::Value(DexValue::Long(10)))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_and_add,
            Some(&test_invocation(vec![
                value,
                DexValue::Long(-3),
                DexValue::Long(-3)
            ]))
        ),
        Some(DexExecutionResult::Value(DexValue::Long(15)))
    );
}

#[test]
fn libcore_thread_pool_executor_submit_posts_a_future_task() {
    // cite: libcore AbstractExecutorService.java#submit
    let program = DexProgram::new(Vec::new());
    let executor = program.allocate_object("Ljava/util/concurrent/ThreadPoolExecutor;".to_owned());
    let command = program.allocate_object("Lcom/example/Task;".to_owned());
    let submit = builtin_method(
        "Ljava/util/concurrent/ThreadPoolExecutor;",
        "submit",
        "Ljava/util/concurrent/Future;",
        &["Ljava/lang/Runnable;"],
    );
    let result = program.invoke_builtin_method(
        &submit,
        Some(&test_invocation(vec![executor.clone(), command.clone()])),
    );
    let Some(DexExecutionResult::Value(task)) = result else {
        panic!("submit should return a future");
    };
    assert!(program.value_matches_type(&task, "Ljava/util/concurrent/FutureTask;"));
    assert_eq!(
        program.read_stored_instance_field(&task, &future_task_runnable_field()),
        Some(command)
    );
    assert!(program
        .runtime
        .borrow()
        .handler_posts
        .iter()
        .any(|post| post.runnable == task && post.delay_millis == 0));
    let result = program.invoke_builtin_method(
        &submit,
        Some(&test_invocation(vec![executor, DexValue::Null])),
    );
    assert!(program.value_matches_type(
        &thrown_exception(result),
        "Ljava/lang/NullPointerException;"
    ));
}

#[test]
fn libcore_schedule_with_fixed_delay_posts_and_repeats_a_periodic_task() {
    // cite: libcore ScheduledThreadPoolExecutor.java#scheduleWithFixedDelay, reExecutePeriodic
    let program = DexProgram::new(Vec::new());
    let executor =
        program.allocate_object("Ljava/util/concurrent/ScheduledThreadPoolExecutor;".to_owned());
    let command = program.allocate_object("Lcom/example/Task;".to_owned());
    let schedule = builtin_method(
        "Ljava/util/concurrent/ScheduledThreadPoolExecutor;",
        "scheduleWithFixedDelay",
        "Ljava/util/concurrent/ScheduledFuture;",
        &[
            "Ljava/lang/Runnable;",
            "J",
            "J",
            "Ljava/util/concurrent/TimeUnit;",
        ],
    );
    let result = program.invoke_builtin_method(
        &schedule,
        Some(&test_invocation(vec![
            executor.clone(),
            command.clone(),
            DexValue::Long(1),
            DexValue::Long(1),
            DexValue::Long(2),
            DexValue::Long(2),
            seconds_unit(),
        ])),
    );
    let Some(DexExecutionResult::Value(task)) = result else {
        panic!("scheduleWithFixedDelay should return the task");
    };
    assert!(program.value_matches_type(&task, "Ljava/util/concurrent/ScheduledFuture;"));
    assert_eq!(
        program.read_stored_instance_field(&task, &future_task_runnable_field()),
        Some(command.clone())
    );
    assert_eq!(
        program.read_stored_instance_field(&task, &scheduled_future_task_period_field()),
        Some(DexValue::Long(-2_000_000_000))
    );
    assert!(program
        .runtime
        .borrow()
        .handler_posts
        .iter()
        .any(|post| post.runnable == task && post.delay_millis == 1000));
    assert_eq!(
        program.repeating_scheduled_task_delay_millis(&task),
        Some(2000)
    );

    let finished = program.allocate_object("Ljava/util/concurrent/FutureTask;".to_owned());
    assert!(program.write_stored_instance_field(
        &finished,
        &future_task_done_field(),
        DexValue::Int(1)
    ));
    assert!(program.write_stored_instance_field(&task, &future_task_runnable_field(), finished));
    program
        .dispatch_runnable(&task, "ScheduledFutureTask.run", 0)
        .expect("periodic task should run");
    assert_eq!(
        program.read_stored_instance_field(&task, &future_task_done_field()),
        None
    );
    assert_eq!(
        program.repeating_scheduled_task_delay_millis(&task),
        Some(2000)
    );

    for (delay, initial) in [(0i64, 1i64), (1, 0)] {
        let result = program.invoke_builtin_method(
            &schedule,
            Some(&test_invocation(vec![
                executor.clone(),
                command.clone(),
                DexValue::Long(initial),
                DexValue::Long(initial),
                DexValue::Long(delay),
                DexValue::Long(delay),
                seconds_unit(),
            ])),
        );
        if delay <= 0 {
            assert!(program.value_matches_type(
                &thrown_exception(result),
                "Ljava/lang/IllegalArgumentException;"
            ));
        } else {
            assert!(matches!(result, Some(DexExecutionResult::Value(_))));
        }
    }
}

// cite: libcore ScheduledThreadPoolExecutor.java#scheduleAtFixedRate
#[test]
fn libcore_schedule_at_fixed_rate_posts_a_positive_periodic_task() {
    let program = DexProgram::new(Vec::new());
    let executor =
        program.allocate_object("Ljava/util/concurrent/ScheduledThreadPoolExecutor;".to_owned());
    let command = program.allocate_object("Lcom/example/Task;".to_owned());
    let schedule = builtin_method(
        "Ljava/util/concurrent/ScheduledThreadPoolExecutor;",
        "scheduleAtFixedRate",
        "Ljava/util/concurrent/ScheduledFuture;",
        &[
            "Ljava/lang/Runnable;",
            "J",
            "J",
            "Ljava/util/concurrent/TimeUnit;",
        ],
    );
    let result = program.invoke_builtin_method(
        &schedule,
        Some(&test_invocation(vec![
            executor.clone(),
            command.clone(),
            DexValue::Long(1),
            DexValue::Long(1),
            DexValue::Long(2),
            DexValue::Long(2),
            seconds_unit(),
        ])),
    );
    let Some(DexExecutionResult::Value(task)) = result else {
        panic!("scheduleAtFixedRate should return the task");
    };
    assert_eq!(
        program.read_stored_instance_field(&task, &future_task_runnable_field()),
        Some(command)
    );
    assert_eq!(
        program.read_stored_instance_field(&task, &scheduled_future_task_period_field()),
        Some(DexValue::Long(2_000_000_000))
    );
    assert_eq!(
        program.repeating_scheduled_task_delay_millis(&task),
        Some(2000)
    );

    let zero_period = program.invoke_builtin_method(
        &schedule,
        Some(&test_invocation(vec![
            executor,
            program.allocate_object("Lcom/example/Task;".to_owned()),
            DexValue::Long(1),
            DexValue::Long(1),
            DexValue::Long(0),
            DexValue::Long(0),
            seconds_unit(),
        ])),
    );
    assert!(program.value_matches_type(
        &thrown_exception(zero_period),
        "Ljava/lang/IllegalArgumentException;"
    ));
}

#[test]
fn libcore_scheduled_future_task_runs_its_runnable_through_future_task_run() {
    // cite: libcore ScheduledThreadPoolExecutor.java#ScheduledFutureTask
    // cite: libcore FutureTask.java#run
    let program = DexProgram::new(Vec::new());
    let command = program.allocate_object("Ljava/util/concurrent/FutureTask;".to_owned());
    assert!(program.write_stored_instance_field(
        &command,
        &future_task_done_field(),
        DexValue::Int(1)
    ));
    let task = program.allocate_object(
        "Ljava/util/concurrent/ScheduledThreadPoolExecutor$ScheduledFutureTask;".to_owned(),
    );
    assert!(program.write_stored_instance_field(&task, &future_task_runnable_field(), command));

    program
        .dispatch_runnable(&task, "ScheduledFutureTask.run", 0)
        .expect("ScheduledFutureTask should run as a FutureTask");

    assert_eq!(
        program.read_stored_instance_field(&task, &future_task_done_field()),
        Some(DexValue::Int(1))
    );
}

// cite: libcore ReentrantReadWriteLock.java#ReentrantReadWriteLock, readLock, writeLock, readerLock, writerLock
#[test]
fn reentrant_read_write_lock_hands_out_the_same_read_and_write_lock_per_instance() {
    let program = DexProgram::new(Vec::new());
    let lock_class = "Ljava/util/concurrent/locks/ReentrantReadWriteLock;";
    let method = |name: &str, return_type: &str| DexMethodId {
        class_descriptor: lock_class.to_owned(),
        name: name.to_owned(),
        proto: crate::dex::DexProto {
            shorty: if name == "<init>" { "V" } else { "L" }.to_owned(),
            return_type: return_type.to_owned(),
            parameters: Vec::new(),
        },
    };
    let init = method("<init>", "V");
    let read_lock = method(
        "readLock",
        "Ljava/util/concurrent/locks/ReentrantReadWriteLock$ReadLock;",
    );
    let write_lock = method(
        "writeLock",
        "Ljava/util/concurrent/locks/ReentrantReadWriteLock$WriteLock;",
    );
    let lock = program.allocate_object(lock_class.to_owned());
    let call = |method: &DexMethodId| {
        program.invoke_builtin_method(method, Some(&test_invocation(vec![lock.clone()])))
    };

    assert_eq!(call(&read_lock), None);
    assert_eq!(call(&init), Some(DexExecutionResult::Void));
    let Some(DexExecutionResult::Value(reader)) = call(&read_lock) else {
        panic!("readLock should return the reader lock");
    };
    let Some(DexExecutionResult::Value(writer)) = call(&write_lock) else {
        panic!("writeLock should return the writer lock");
    };

    assert!(program.value_matches_type(
        &reader,
        "Ljava/util/concurrent/locks/ReentrantReadWriteLock$ReadLock;"
    ));
    assert!(program.value_matches_type(&writer, "Ljava/util/concurrent/locks/Lock;"));
    assert_ne!(reader, writer);
    let bridge_read_lock = method("readLock", "Ljava/util/concurrent/locks/Lock;");
    assert_eq!(
        call(&bridge_read_lock),
        Some(DexExecutionResult::Value(reader.clone()))
    );
    assert_eq!(call(&read_lock), Some(DexExecutionResult::Value(reader)));
    assert_eq!(call(&write_lock), Some(DexExecutionResult::Value(writer)));
}

#[test]
fn libcore_reentrant_lock_new_condition_returns_a_condition_object() {
    // cite: libcore ReentrantLock.java#newCondition, ConditionObject
    let program = DexProgram::new(Vec::new());
    let new_condition = builtin_method(
        "Ljava/util/concurrent/locks/ReentrantLock;",
        "newCondition",
        "Ljava/util/concurrent/locks/Condition;",
        &[],
    );
    let lock = program.allocate_object("Ljava/util/concurrent/locks/ReentrantLock;".to_owned());
    let Some(DexExecutionResult::Value(condition)) =
        program.invoke_builtin_method(&new_condition, Some(&test_invocation(vec![lock])))
    else {
        panic!("newCondition should return a condition");
    };

    assert!(program.value_matches_type(&condition, "Ljava/util/concurrent/locks/Condition;"));
    assert_eq!(
        value_class_descriptor(&condition),
        Some("Ljava/util/concurrent/locks/ReentrantLock$ConditionObject;")
    );
}

#[test]
fn libcore_semaphore_constructor_stores_permits_and_rejects_negative_counts() {
    // cite: libcore Semaphore.java#Semaphore
    let program = DexProgram::new(Vec::new());
    let constructor = builtin_method("Ljava/util/concurrent/Semaphore;", "<init>", "V", &["I"]);
    let semaphore = program.allocate_object("Ljava/util/concurrent/Semaphore;".to_owned());

    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![semaphore.clone(), DexValue::Int(2)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .semaphore_permits
            .get(&map_identity_key(&semaphore)),
        Some(&2)
    );
    let result = program.invoke_builtin_method(
        &constructor,
        Some(&test_invocation(vec![
            program.allocate_object("Ljava/util/concurrent/Semaphore;".to_owned()),
            DexValue::Int(-1),
        ])),
    );
    assert!(program.value_matches_type(
        &thrown_exception(result),
        "Ljava/lang/IllegalArgumentException;"
    ));
}

#[test]
fn libcore_soft_reference_constructor_keeps_the_referent() {
    // cite: libcore SoftReference.java#SoftReference
    let program = DexProgram::new(Vec::new());
    let constructor = builtin_method(
        "Ljava/lang/ref/SoftReference;",
        "<init>",
        "V",
        &["Ljava/lang/Object;"],
    );
    let get = builtin_method(
        "Ljava/lang/ref/Reference;",
        "get",
        "Ljava/lang/Object;",
        &[],
    );
    let reference = program.allocate_object("Ljava/lang/ref/SoftReference;".to_owned());
    let referent = program.allocate_object("Ljava/lang/Object;".to_owned());

    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![reference.clone(), referent.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&get, Some(&test_invocation(vec![reference]))),
        Some(DexExecutionResult::Value(referent))
    );
}

#[test]
fn libcore_condition_await_parks_and_signal_all_resumes_the_waiter() {
    // cite: libcore Condition.java#await, signalAll
    let program = DexProgram::new(Vec::new());
    let await_method = builtin_method("Ljava/util/concurrent/locks/Condition;", "await", "V", &[]);
    let signal_all = builtin_method(
        "Ljava/util/concurrent/locks/Condition;",
        "signalAll",
        "V",
        &[],
    );
    let condition = program
        .allocate_object("Ljava/util/concurrent/locks/ReentrantLock$ConditionObject;".to_owned());
    let worker = program.allocate_object("Ljava/lang/Thread;".to_owned());

    let parked = program.with_current_guest_thread(worker.clone(), || {
        program.invoke_builtin_method(
            &await_method,
            Some(&test_invocation(vec![condition.clone()])),
        )
    });
    assert!(matches!(parked, Some(DexExecutionResult::Parked(_))));
    assert_eq!(
        program
            .runtime
            .borrow()
            .condition_waiters
            .get(&map_identity_key(&condition))
            .map(Vec::len),
        Some(1)
    );
    assert_eq!(
        program.invoke_builtin_method(&signal_all, Some(&test_invocation(vec![condition.clone()]))),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .condition_waiters
            .get(&map_identity_key(&condition)),
        None
    );
    assert_eq!(
        program.invoke_builtin_method(&signal_all, Some(&test_invocation(vec![condition]))),
        Some(DexExecutionResult::Void)
    );
}

// cite: libcore AtomicInteger.java#getAndSet, compareAndSet, incrementAndGet
#[test]
fn libcore_atomic_integer_get_and_set_compare_and_set_and_wrapping_increment() {
    let program = DexProgram::new(Vec::new());
    let class = "Ljava/util/concurrent/atomic/AtomicInteger;";
    let init = builtin_method(class, "<init>", "V", &["I"]);
    let get_and_set = builtin_method(class, "getAndSet", "I", &["I"]);
    let compare_and_set = builtin_method(class, "compareAndSet", "Z", &["I", "I"]);
    let increment_and_get = builtin_method(class, "incrementAndGet", "I", &[]);
    let get = builtin_method(class, "get", "I", &[]);
    let atomic = program.allocate_object(class.to_owned());
    let call = |method: &DexMethodId, mut arguments: Vec<DexValue>| {
        arguments.insert(0, atomic.clone());
        program.invoke_builtin_method(method, Some(&test_invocation(arguments)))
    };

    call(&init, vec![DexValue::Int(5)]);
    assert_eq!(
        call(&get_and_set, vec![DexValue::Int(9)]),
        Some(DexExecutionResult::Int(5))
    );
    assert_eq!(
        call(&compare_and_set, vec![DexValue::Int(1), DexValue::Int(2)]),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        call(
            &compare_and_set,
            vec![DexValue::Int(9), DexValue::Int(i32::MAX)]
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        call(&increment_and_get, vec![]),
        Some(DexExecutionResult::Int(i32::MIN))
    );
    assert_eq!(call(&get, vec![]), Some(DexExecutionResult::Int(i32::MIN)));
}

// cite: libcore FutureTask.java#get
#[test]
fn future_get_intrinsic_routes_timed_get_to_the_task_outcome() {
    let program = DexProgram::new(Vec::new());
    let task = program.allocate_object("Ljava/util/concurrent/FutureTask;".to_owned());
    let field = |name: &str, type_descriptor: &str| DexFieldId {
        class_descriptor: "Ljava/util/concurrent/FutureTask;".to_owned(),
        type_descriptor: type_descriptor.to_owned(),
        name: name.to_owned(),
    };
    let value = DexValue::String(std::rc::Rc::from("done"));
    program.write_stored_instance_field(&task, &field("done", "Z"), DexValue::Int(1));
    program.write_stored_instance_field(
        &task,
        &field("outcome", "Ljava/lang/Object;"),
        value.clone(),
    );
    let timed = builtin_method(
        "Ljava/util/concurrent/Future;",
        "get",
        "Ljava/lang/Object;",
        &["J", "Ljava/util/concurrent/TimeUnit;"],
    );
    let other = builtin_method("Ljava/util/concurrent/Future;", "isDone", "Z", &[]);
    let invocation = test_invocation(vec![
        task,
        DexValue::Long(5),
        DexValue::Long(5),
        DexValue::Reference(std::rc::Rc::from("Ljava/util/concurrent/TimeUnit;:SECONDS")),
    ]);

    assert_eq!(
        program
            .future_get_intrinsic(&timed, &invocation, 0)
            .unwrap(),
        Some(DexExecutionResult::Value(value))
    );
    assert_eq!(
        program
            .future_get_intrinsic(&other, &invocation, 0)
            .unwrap(),
        None
    );
}

// cite: libcore AtomicInteger.java#getAndDecrement, intValue, longValue, decrementAndGet
#[test]
fn libcore_atomic_integer_decrement_and_number_views_wrap_like_java() {
    let program = DexProgram::new(Vec::new());
    let class = "Ljava/util/concurrent/atomic/AtomicInteger;";
    let init = builtin_method(class, "<init>", "V", &["I"]);
    let get_and_decrement = builtin_method(class, "getAndDecrement", "I", &[]);
    let decrement_and_get = builtin_method(class, "decrementAndGet", "I", &[]);
    let int_value = builtin_method(class, "intValue", "I", &[]);
    let long_value = builtin_method(class, "longValue", "J", &[]);
    let atomic = program.allocate_object(class.to_owned());
    let call = |method: &DexMethodId, mut arguments: Vec<DexValue>| {
        arguments.insert(0, atomic.clone());
        program.invoke_builtin_method(method, Some(&test_invocation(arguments)))
    };

    call(&init, vec![DexValue::Int(i32::MIN + 1)]);
    assert_eq!(
        call(&get_and_decrement, vec![]),
        Some(DexExecutionResult::Int(i32::MIN + 1))
    );
    assert_eq!(
        call(&decrement_and_get, vec![]),
        Some(DexExecutionResult::Int(i32::MAX))
    );
    assert_eq!(
        call(&int_value, vec![]),
        Some(DexExecutionResult::Int(i32::MAX))
    );
    assert_eq!(
        call(&long_value, vec![]),
        Some(DexExecutionResult::Value(DexValue::Long(i64::from(
            i32::MAX
        ))))
    );
}

// cite: libcore AtomicLong.java#lazySet, getAndSet, compareAndSet, incrementAndGet, getAndDecrement, addAndGet
#[test]
fn libcore_atomic_long_updates_wrap_like_java() {
    let program = DexProgram::new(Vec::new());
    let class = "Ljava/util/concurrent/atomic/AtomicLong;";
    let atomic = program.allocate_object(class.to_owned());
    let wide = |value: i64| [DexValue::Long(value), DexValue::Long(value)];
    let call = |name: &str, ret: &str, params: &[&str], arguments: Vec<DexValue>| {
        let mut values = vec![atomic.clone()];
        values.extend(arguments);
        program.invoke_builtin_method(
            &builtin_method(class, name, ret, params),
            Some(&test_invocation(values)),
        )
    };
    let long = |value: i64| Some(DexExecutionResult::Value(DexValue::Long(value)));

    assert_eq!(
        call("lazySet", "V", &["J"], wide(i64::MAX).to_vec()),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(call("incrementAndGet", "J", &[], vec![]), long(i64::MIN));
    assert_eq!(
        call("getAndSet", "J", &["J"], wide(5).to_vec()),
        long(i64::MIN)
    );
    let mut swap = wide(4).to_vec();
    swap.extend(wide(9));
    assert_eq!(
        call("compareAndSet", "Z", &["J", "J"], swap),
        Some(DexExecutionResult::Int(0))
    );
    let mut swap = wide(5).to_vec();
    swap.extend(wide(9));
    assert_eq!(
        call("compareAndSet", "Z", &["J", "J"], swap),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(call("getAndDecrement", "J", &[], vec![]), long(9));
    assert_eq!(call("addAndGet", "J", &["J"], wide(10).to_vec()), long(18));
    assert_eq!(call("get", "J", &[], vec![]), long(18));
}

fn future_contract_result(method: &str) -> DexExecutionResult {
    let dex_file = crate::dex::parse_dex_file(
        "future_task_contract.dex",
        include_bytes!("fixtures/future_task_contract.dex").to_vec(),
    )
    .expect("FutureTask contract DEX should parse");
    let program = DexProgram::new(vec![dex_file]);
    let method = DexMethodId {
        class_descriptor: "Lwinedroid/tests/FutureContract;".to_owned(),
        name: method.to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let resolved = program
        .resolve_exact_method(&method)
        .expect("FutureTask contract method should resolve")
        .expect("FutureTask contract method should exist");

    program
        .execute_method(None, &resolved, None, 0)
        .expect("FutureTask contract method should execute")
}

// cite: libcore FutureTask.java#run, set, done, finishCompletion
#[test]
fn libcore_future_task_subclass_run_calls_overridden_set_then_done_once() {
    assert_eq!(
        future_contract_result("runValue"),
        DexExecutionResult::Value(DexValue::String(
            "run;set:value;done;ran;run;ran;value".into()
        ))
    );
}

// cite: libcore FutureTask.java#run, setException, done, report
#[test]
fn libcore_future_task_subclass_failure_reaches_overridden_set_exception() {
    assert_eq!(
        future_contract_result("runFailure"),
        DexExecutionResult::Value(DexValue::String("run;ex:boom;done;ran;boom".into()))
    );
}

fn future_contract_static(
    program: &DexProgram,
    method: &str,
    return_type: &str,
) -> DexExecutionResult {
    let method = DexMethodId {
        class_descriptor: "Lwinedroid/tests/FutureContract;".to_owned(),
        name: method.to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: return_type.to_owned(),
            parameters: Vec::new(),
        },
    };
    let resolved = program.resolve_exact_method(&method).unwrap().unwrap();
    program.execute_method(None, &resolved, None, 0).unwrap()
}

// cite: libcore FutureTask.java#run, setException
// cite: libcore Executors.java#RunnableAdapter
#[test]
fn libcore_future_task_runnable_failure_reaches_set_exception_and_get() {
    assert_eq!(
        future_contract_result("runnableFailure"),
        DexExecutionResult::Value(DexValue::String("ex:bad;bad".into()))
    );
}

// cite: libcore FutureTask.java#cancel, finishCompletion, done
#[test]
fn libcore_future_task_cancel_runs_the_done_override_once() {
    assert_eq!(
        future_contract_result("cancelRunsDoneOnce"),
        DexExecutionResult::Value(DexValue::String("done;true:false:true".into()))
    );
}

// cite: libcore FutureTask.java#run, done
#[test]
fn libcore_future_task_subclass_without_run_override_runs_when_dispatched() {
    let dex_file = crate::dex::parse_dex_file(
        "future_task_contract.dex",
        include_bytes!("fixtures/future_task_contract.dex").to_vec(),
    )
    .unwrap();
    let program = DexProgram::new(vec![dex_file]);
    let DexExecutionResult::Value(task) = future_contract_static(
        &program,
        "plainTask",
        "Lwinedroid/tests/FutureContract$Plain;",
    ) else {
        panic!("plainTask should return a task");
    };

    program.dispatch_runnable(&task, "test", 0).unwrap();

    assert_eq!(
        future_contract_static(&program, "log", "Ljava/lang/String;"),
        DexExecutionResult::Value(DexValue::String("call;done;".into()))
    );
}
