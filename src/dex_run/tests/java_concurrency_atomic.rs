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
            &thread_pool_executor_field("handler", "Ljava/util/concurrent/RejectedExecutionHandler;")
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
        let Some(DexExecutionResult::Throw(exception)) = program
            .invoke_builtin_method(&thread_pool_executor_constructor(), Some(&invocation))
        else {
            panic!("constructor should throw {expected}");
        };

        assert!(program.value_matches_type(&exception, expected), "{expected}");
    };
    let illegal = "Ljava/lang/IllegalArgumentException;";
    let null_pointer = "Ljava/lang/NullPointerException;";

    throws(-1, 4, 1, seconds_unit(), queue.clone(), factory.clone(), illegal);
    throws(1, 0, 1, seconds_unit(), queue.clone(), factory.clone(), illegal);
    throws(5, 4, 1, seconds_unit(), queue.clone(), factory.clone(), illegal);
    throws(1, 4, -1, seconds_unit(), queue.clone(), factory.clone(), illegal);
    throws(1, 4, 1, seconds_unit(), DexValue::Null, factory.clone(), null_pointer);
    throws(1, 4, 1, seconds_unit(), queue.clone(), DexValue::Null, null_pointer);
    throws(1, 4, 1, DexValue::Null, queue.clone(), factory.clone(), null_pointer);
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
