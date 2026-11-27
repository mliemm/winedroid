use super::super::*;

impl DexProgram {
    pub(super) fn invoke_java_concurrency_atomic_builtin(
        &self,
        method: &DexMethodId,
        invocation: Option<&DexInvocation>,
    ) -> Option<DexExecutionResult> {
        if method.matches_builtin_signature(
            "Ljava/util/concurrent/Executors;",
            "newFixedThreadPool",
            "(ILjava/util/concurrent/ThreadFactory;)Ljava/util/concurrent/ExecutorService;",
        ) {
            let invocation = invocation?;
            let value = fixed_thread_pool_reference(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.matches_any_builtin_signature(
            "Ljava/util/concurrent/ThreadPoolExecutor;",
            "<init>",
            &[
                "(IIJLjava/util/concurrent/TimeUnit;Ljava/util/concurrent/BlockingQueue;Ljava/util/concurrent/ThreadFactory;)V",
                "(IIJLjava/util/concurrent/TimeUnit;Ljava/util/concurrent/BlockingQueue;Ljava/util/concurrent/ThreadFactory;Ljava/util/concurrent/RejectedExecutionHandler;)V",
            ],
        ) {
            let invocation = invocation?;

            return self.thread_pool_executor_init(invocation);
        }

        // cite: libcore ScheduledThreadPoolExecutor.java#ScheduledThreadPoolExecutor, DEFAULT_KEEPALIVE_MILLIS, DelayedWorkQueue
        if method.matches_builtin_signature(
            "Ljava/util/concurrent/ScheduledThreadPoolExecutor;",
            "<init>",
            "(ILjava/util/concurrent/ThreadFactory;)V",
        ) {
            let invocation = invocation?;

            return self.scheduled_thread_pool_executor_init(invocation);
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/ThreadPoolExecutor;",
            "allowCoreThreadTimeOut",
            "(Z)V",
        ) {
            let invocation = invocation?;

            return self.thread_pool_executor_allow_core_thread_time_out(invocation);
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/ThreadPoolExecutor;",
            "submit",
            "(Ljava/lang/Runnable;)Ljava/util/concurrent/Future;",
        ) {
            // cite: libcore AbstractExecutorService.java#submit
            let invocation = invocation?;

            return self.thread_pool_executor_submit_runnable(invocation);
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/ThreadPoolExecutor;",
            "submit",
            "(Ljava/util/concurrent/Callable;)Ljava/util/concurrent/Future;",
        ) {
            // cite: libcore AbstractExecutorService.java#submit
            let invocation = invocation?;

            return self.thread_pool_executor_submit_callable(invocation);
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/ScheduledThreadPoolExecutor;",
            "submit",
            "(Ljava/lang/Runnable;)Ljava/util/concurrent/Future;",
        ) {
            // cite: libcore ScheduledThreadPoolExecutor.java#submit
            let invocation = invocation?;
            let [executor, runnable] = invocation.arguments.as_slice() else {
                return None;
            };
            let scheduled = DexInvocation {
                arguments: smallvec::smallvec![
                    executor.clone(),
                    runnable.clone(),
                    DexValue::Long(0),
                    DexValue::Long(0),
                    DexValue::Reference(Rc::from("Ljava/util/concurrent/TimeUnit;.NANOSECONDS")),
                ],
                ..invocation.clone()
            };
            return self.scheduled_thread_pool_executor_schedule_runnable(&scheduled);
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/ScheduledThreadPoolExecutor;",
            "schedule",
            "(Ljava/lang/Runnable;JLjava/util/concurrent/TimeUnit;)Ljava/util/concurrent/ScheduledFuture;",
        ) {
            let invocation = invocation?;

            return self.scheduled_thread_pool_executor_schedule_runnable(invocation);
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/ScheduledThreadPoolExecutor;",
            "scheduleWithFixedDelay",
            "(Ljava/lang/Runnable;JJLjava/util/concurrent/TimeUnit;)Ljava/util/concurrent/ScheduledFuture;",
        ) {
            // cite: libcore ScheduledThreadPoolExecutor.java#scheduleWithFixedDelay
            let invocation = invocation?;

            return self.scheduled_thread_pool_executor_schedule_with_fixed_delay(invocation);
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/ScheduledThreadPoolExecutor;",
            "scheduleAtFixedRate",
            "(Ljava/lang/Runnable;JJLjava/util/concurrent/TimeUnit;)Ljava/util/concurrent/ScheduledFuture;",
        ) {
            // cite: libcore ScheduledThreadPoolExecutor.java#scheduleAtFixedRate
            let invocation = invocation?;

            return self.scheduled_thread_pool_executor_schedule_at_fixed_rate(invocation);
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/ThreadPoolExecutor;",
            "setKeepAliveTime",
            "(JLjava/util/concurrent/TimeUnit;)V",
        ) {
            let invocation = invocation?;

            return self.thread_pool_executor_set_keep_alive_time(invocation);
        }

        if method.class_descriptor == "Ljava/util/concurrent/FutureTask;"
            && method.name == "<init>"
            && method
                .proto
                .descriptor_is("(Ljava/util/concurrent/Callable;)V")
        {
            let invocation = invocation?;
            self.future_task_init(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/util/concurrent/FutureTask;"
            && method.name == "<init>"
            && method
                .proto
                .descriptor_is("(Ljava/lang/Runnable;Ljava/lang/Object;)V")
        {
            let invocation = invocation?;
            self.future_task_init_runnable(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/util/concurrent/FutureTask;"
            && method.name == "cancel"
            && method.proto.descriptor_is("(Z)Z")
        {
            let invocation = invocation?;
            let value = self.future_task_cancel(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Ljava/util/concurrent/FutureTask;"
            && method.name == "isCancelled"
            && method.proto.descriptor_is("()Z")
        {
            let invocation = invocation?;
            let value = self.future_task_is_cancelled(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Ljava/util/concurrent/FutureTask;"
            && method.name == "isDone"
            && method.proto.descriptor_is("()Z")
        {
            let invocation = invocation?;
            let value = self.future_task_is_done(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Ljava/util/concurrent/Executors;"
            && method.name == "newCachedThreadPool"
            && matches!(
                method.proto.descriptor().as_str(),
                "()Ljava/util/concurrent/ExecutorService;"
                    | "(Ljava/util/concurrent/ThreadFactory;)Ljava/util/concurrent/ExecutorService;"
            )
        {
            // cite: libcore Executors.java#newCachedThreadPool
            return Some(DexExecutionResult::Value(DexValue::Reference(
                std::rc::Rc::from("Ljava/util/concurrent/ExecutorService;.cached"),
            )));
        }

        if method.class_descriptor == "Ljava/util/concurrent/Executors;"
            && method.name == "newSingleThreadExecutor"
            && matches!(
                method.proto.descriptor().as_str(),
                "()Ljava/util/concurrent/ExecutorService;"
                    | "(Ljava/util/concurrent/ThreadFactory;)Ljava/util/concurrent/ExecutorService;"
            )
        {
            return Some(DexExecutionResult::Value(DexValue::Reference(
                std::rc::Rc::from("Ljava/util/concurrent/ExecutorService;.single"),
            )));
        }

        if method.class_descriptor == "Ljava/util/concurrent/Executors;"
            && method.name == "newSingleThreadScheduledExecutor"
            && matches!(
                method.proto.descriptor().as_str(),
                "()Ljava/util/concurrent/ScheduledExecutorService;"
                    | "(Ljava/util/concurrent/ThreadFactory;)Ljava/util/concurrent/ScheduledExecutorService;"
            )
        {
            return Some(DexExecutionResult::Value(DexValue::Reference(
                std::rc::Rc::from("Ljava/util/concurrent/ScheduledExecutorService;.single"),
            )));
        }

        if matches!(
            method.class_descriptor.as_str(),
            "Ljava/util/concurrent/Executor;"
                | "Ljava/util/concurrent/ExecutorService;"
                | "Ljava/util/concurrent/ScheduledExecutorService;"
                | "Ljava/util/concurrent/ThreadPoolExecutor;"
        ) && method.name == "execute"
            && method.proto.descriptor_is("(Ljava/lang/Runnable;)V")
        {
            let invocation = invocation?;
            if self.is_thread_pool_executor(invocation.arguments.first()?)
                && invocation.arguments.get(1) == Some(&DexValue::Null)
            {
                // cite: libcore ThreadPoolExecutor.java#execute
                return Some(DexExecutionResult::Throw(
                    self.allocate_object("Ljava/lang/NullPointerException;".to_owned()),
                ));
            }
            self.executor_execute(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if matches!(
            method.class_descriptor.as_str(),
            "Ljava/util/concurrent/ExecutorService;"
                | "Ljava/util/concurrent/ScheduledExecutorService;"
                | "Ljava/util/concurrent/ThreadPoolExecutor;"
        ) && method.name == "isShutdown"
            && method.proto.descriptor_is("()Z")
        {
            let invocation = invocation?;
            let value = self.executor_service_is_shutdown(invocation)?;

            return Some(DexExecutionResult::Int(i32::from(value)));
        }

        if matches!(
            method.class_descriptor.as_str(),
            "Ljava/util/concurrent/ExecutorService;"
                | "Ljava/util/concurrent/ScheduledExecutorService;"
                | "Ljava/util/concurrent/ThreadPoolExecutor;"
        ) && method.name == "shutdown"
            && method.proto.descriptor_is("()V")
        {
            let invocation = invocation?;
            self.executor_service_shutdown(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if matches!(
            method.class_descriptor.as_str(),
            "Ljava/util/concurrent/ExecutorService;"
                | "Ljava/util/concurrent/ScheduledExecutorService;"
                | "Ljava/util/concurrent/ThreadPoolExecutor;"
        ) && method.name == "shutdownNow"
            && method.proto.descriptor_is("()Ljava/util/List;")
        {
            let invocation = invocation?;
            self.executor_service_shutdown(invocation)?;

            return Some(DexExecutionResult::Value(self.collections_empty_list()));
        }

        if matches!(
            method.class_descriptor.as_str(),
            "Ljava/util/concurrent/ExecutorService;"
                | "Ljava/util/concurrent/ScheduledExecutorService;"
                | "Ljava/util/concurrent/ThreadPoolExecutor;"
        ) && method.name == "isTerminated"
            && method.proto.descriptor_is("()Z")
        {
            let invocation = invocation?;
            let value = self.executor_service_is_terminated(invocation)?;

            return Some(DexExecutionResult::Int(i32::from(value)));
        }

        if matches!(
            method.class_descriptor.as_str(),
            "Ljava/util/concurrent/ExecutorService;"
                | "Ljava/util/concurrent/ScheduledExecutorService;"
                | "Ljava/util/concurrent/ThreadPoolExecutor;"
        ) && method.name == "awaitTermination"
            && method
                .proto
                .descriptor_is("(JLjava/util/concurrent/TimeUnit;)Z")
        {
            let invocation = invocation?;
            let value = self.executor_service_is_terminated(invocation)?;

            return Some(DexExecutionResult::Int(i32::from(value)));
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/locks/LockSupport;",
            "parkNanos",
            "(J)V",
        ) {
            let invocation = invocation?;
            let should_park = lock_support_park_nanos(invocation)?;

            if !should_park {
                return Some(DexExecutionResult::Void);
            }

            let DexValue::Long(nanos) = invocation.arguments[0] else {
                return None;
            };

            return Some(DexExecutionResult::Parked(
                DexParkedExecution::with_park_timeout(nanos),
            ));
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/locks/LockSupport;",
            "parkNanos",
            "(Ljava/lang/Object;J)V",
        ) {
            let invocation = invocation?;
            let should_park = lock_support_park_nanos_with_blocker(invocation)?;

            if !should_park {
                return Some(DexExecutionResult::Void);
            }

            let DexValue::Long(nanos) = invocation.arguments[1] else {
                return None;
            };

            return Some(DexExecutionResult::Parked(
                DexParkedExecution::with_park_timeout(nanos),
            ));
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/locks/LockSupport;",
            "unpark",
            "(Ljava/lang/Thread;)V",
        ) {
            let invocation = invocation?;
            self.lock_support_unpark(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        // cite: libcore AbstractExecutorService.java#AbstractExecutorService
        if method.matches_builtin_signature(
            "Ljava/util/concurrent/AbstractExecutorService;",
            "<init>",
            "()V",
        ) {
            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/util/concurrent/locks/ReentrantLock;"
            && method.name == "<init>"
            && matches!(method.proto.descriptor().as_str(), "()V" | "(Z)V")
        {
            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/util/concurrent/locks/ReentrantReadWriteLock;"
            && method.name == "<init>"
            && matches!(method.proto.descriptor().as_str(), "()V" | "(Z)V")
        {
            let invocation = invocation?;
            self.reentrant_read_write_lock_init(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if matches!(
            method.class_descriptor.as_str(),
            "Ljava/util/concurrent/locks/ReentrantReadWriteLock;"
                | "Ljava/util/concurrent/locks/ReadWriteLock;"
        ) && matches!(method.name.as_str(), "readLock" | "writeLock")
        {
            let invocation = invocation?;
            let [lock] = invocation.arguments.as_slice() else {
                return None;
            };
            let (field, return_type) = reentrant_read_write_lock_accessor(&method.name)?;
            if !method.proto.descriptor_is(&format!("(){return_type}"))
                && !method
                    .proto
                    .descriptor_is("()Ljava/util/concurrent/locks/Lock;")
            {
                return None;
            }

            return Some(DexExecutionResult::Value(
                self.read_stored_instance_field(lock, &field)?,
            ));
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/ThreadPoolExecutor$AbortPolicy;",
            "<init>",
            "()V",
        ) {
            // cite: libcore ThreadPoolExecutor.java#AbortPolicy
            return Some(DexExecutionResult::Void);
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/ThreadPoolExecutor$DiscardOldestPolicy;",
            "<init>",
            "()V",
        ) {
            // cite: libcore ThreadPoolExecutor.java#DiscardOldestPolicy
            return Some(DexExecutionResult::Void);
        }

        if matches!(
            method.class_descriptor.as_str(),
            "Ljava/util/concurrent/locks/Condition;"
                | "Ljava/util/concurrent/locks/ReentrantLock$ConditionObject;"
        ) && matches!(method.name.as_str(), "await" | "awaitUninterruptibly")
            && method.proto.descriptor_is("()V")
        {
            // cite: libcore Condition.java#await
            let invocation = invocation?;
            if !dex_value_is_object_like(&invocation.arguments[0]) {
                return None;
            }
            let thread = self.runtime.borrow().current_guest_thread.clone();
            self.runtime
                .borrow_mut()
                .condition_waiters
                .entry(map_identity_key(&invocation.arguments[0]))
                .or_default()
                .push(thread);

            return Some(DexExecutionResult::Parked(DexParkedExecution::empty()));
        }

        if matches!(
            method.class_descriptor.as_str(),
            "Ljava/util/concurrent/locks/Condition;"
                | "Ljava/util/concurrent/locks/ReentrantLock$ConditionObject;"
        ) && matches!(method.name.as_str(), "signal" | "signalAll")
            && method.proto.descriptor_is("()V")
        {
            // cite: libcore Condition.java#signal, signalAll
            let invocation = invocation?;
            self.condition_signal(invocation, method.name == "signalAll")?;

            return Some(DexExecutionResult::Void);
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/locks/ReentrantLock;",
            "newCondition",
            "()Ljava/util/concurrent/locks/Condition;",
        ) {
            // cite: libcore ReentrantLock.java#newCondition, ConditionObject
            let invocation = invocation?;
            if !dex_value_is_object_like(&invocation.arguments[0]) {
                return None;
            }
            let condition = self.allocate_object(
                "Ljava/util/concurrent/locks/ReentrantLock$ConditionObject;".to_owned(),
            );

            return Some(DexExecutionResult::Value(condition));
        }

        if is_builtin_lock_class(&method.class_descriptor)
            && matches!(method.name.as_str(), "lock" | "unlock")
            && method.proto.descriptor_is("()V")
        {
            return Some(DexExecutionResult::Void);
        }

        if is_builtin_lock_class(&method.class_descriptor)
            && method.name == "tryLock"
            && method.proto.descriptor_is("()Z")
        {
            return Some(DexExecutionResult::Int(1));
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/atomic/AtomicBoolean;",
            "<init>",
            "()V",
        ) {
            let invocation = invocation?;
            self.initialize_atomic_boolean(invocation, 0)?;

            return Some(DexExecutionResult::Void);
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/atomic/AtomicBoolean;",
            "<init>",
            "(Z)V",
        ) {
            let invocation = invocation?;
            let DexValue::Int(initial_value) = invocation.arguments.get(1)? else {
                return None;
            };
            self.initialize_atomic_boolean(invocation, *initial_value)?;

            return Some(DexExecutionResult::Void);
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/atomic/AtomicBoolean;",
            "get",
            "()Z",
        ) {
            let invocation = invocation?;
            let value = self.atomic_boolean_get(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Ljava/util/concurrent/atomic/AtomicBoolean;"
            && matches!(method.name.as_str(), "set" | "lazySet")
            && method.proto.descriptor_is("(Z)V")
        {
            let invocation = invocation?;
            self.atomic_boolean_set(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/atomic/AtomicBoolean;",
            "getAndSet",
            "(Z)Z",
        ) {
            let invocation = invocation?;
            let value = self.atomic_boolean_get_and_set(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Ljava/util/concurrent/atomic/AtomicBoolean;"
            && matches!(method.name.as_str(), "compareAndSet" | "weakCompareAndSet")
            && method.proto.descriptor_is("(ZZ)Z")
        {
            let invocation = invocation?;
            let value = self.atomic_boolean_compare_and_set(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/atomic/AtomicInteger;",
            "<init>",
            "()V",
        ) {
            let invocation = invocation?;
            self.initialize_atomic_integer(invocation, 0)?;

            return Some(DexExecutionResult::Void);
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/atomic/AtomicInteger;",
            "<init>",
            "(I)V",
        ) {
            let invocation = invocation?;
            let DexValue::Int(initial_value) = invocation.arguments.get(1)? else {
                return None;
            };
            self.initialize_atomic_integer(invocation, *initial_value)?;

            return Some(DexExecutionResult::Void);
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/CountDownLatch;",
            "<init>",
            "(I)V",
        ) {
            // cite: libcore CountDownLatch.java#CountDownLatch, Sync, setState
            let invocation = invocation?;
            let DexValue::Int(count) = invocation.arguments.get(1)? else {
                return None;
            };
            if *count < 0 {
                return Some(DexExecutionResult::Throw(
                    self.allocate_object("Ljava/lang/IllegalArgumentException;".to_owned()),
                ));
            }
            self.initialize_atomic_integer(invocation, *count)?;

            return Some(DexExecutionResult::Void);
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/CountDownLatch;",
            "countDown",
            "()V",
        ) {
            // cite: libcore CountDownLatch.java#countDown, tryReleaseShared
            let invocation = invocation?;
            self.count_down_latch(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/atomic/AtomicInteger;",
            "incrementAndGet",
            "()I",
        ) {
            let invocation = invocation?;
            let value = self.increment_atomic_integer(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/atomic/AtomicInteger;",
            "getAndIncrement",
            "()I",
        ) {
            let invocation = invocation?;
            let value = self.get_and_increment_atomic_integer(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/atomic/AtomicInteger;",
            "decrementAndGet",
            "()I",
        ) {
            let invocation = invocation?;
            let value = self.decrement_atomic_integer(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/atomic/AtomicInteger;",
            "get",
            "()I",
        ) {
            let invocation = invocation?;
            let value = self.atomic_integer_get(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Ljava/util/concurrent/atomic/AtomicInteger;"
            && matches!(method.name.as_str(), "set" | "lazySet")
            && method.proto.descriptor_is("(I)V")
        {
            let invocation = invocation?;
            self.atomic_integer_set(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/atomic/AtomicInteger;",
            "addAndGet",
            "(I)I",
        ) {
            let invocation = invocation?;
            let (_previous, current) = self.atomic_integer_add(invocation)?;

            return Some(DexExecutionResult::Int(current));
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/atomic/AtomicInteger;",
            "getAndAdd",
            "(I)I",
        ) {
            let invocation = invocation?;
            let (previous, _current) = self.atomic_integer_add(invocation)?;

            return Some(DexExecutionResult::Int(previous));
        }

        if method.class_descriptor == "Ljava/util/concurrent/atomic/AtomicInteger;"
            && matches!(method.name.as_str(), "compareAndSet" | "weakCompareAndSet")
            && method.proto.descriptor_is("(II)Z")
        {
            let invocation = invocation?;
            let value = self.atomic_integer_compare_and_set(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Ljava/util/concurrent/Semaphore;"
            && method.name == "<init>"
            && matches!(method.proto.descriptor().as_str(), "(I)V" | "(IZ)V")
        {
            // cite: libcore Semaphore.java#Semaphore
            let invocation = invocation?;
            let DexValue::Int(permits) = invocation.arguments.get(1)? else {
                return None;
            };
            if *permits < 0 {
                return Some(DexExecutionResult::Throw(
                    self.allocate_object("Ljava/lang/IllegalArgumentException;".to_owned()),
                ));
            }
            self.runtime
                .borrow_mut()
                .semaphore_permits
                .insert(map_identity_key(&invocation.arguments[0]), *permits);

            return Some(DexExecutionResult::Void);
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/atomic/AtomicLong;",
            "<init>",
            "()V",
        ) {
            // cite: libcore AtomicLong.java#AtomicLong
            let invocation = invocation?;
            self.initialize_atomic_long(invocation, 0)?;

            return Some(DexExecutionResult::Void);
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/atomic/AtomicLong;",
            "<init>",
            "(J)V",
        ) {
            // cite: libcore AtomicLong.java#AtomicLong
            let invocation = invocation?;
            let [object, DexValue::Long(initial_value), DexValue::Long(high_slot_value)] =
                invocation.arguments.as_slice()
            else {
                return None;
            };
            let _ = object;
            if initial_value != high_slot_value {
                return None;
            }
            self.initialize_atomic_long(invocation, *initial_value)?;

            return Some(DexExecutionResult::Void);
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/atomic/AtomicLong;",
            "getAndAdd",
            "(J)J",
        ) {
            // cite: libcore AtomicLong.java#getAndAdd
            let invocation = invocation?;
            let value = self.atomic_long_get_and_add(invocation)?;

            return Some(DexExecutionResult::Value(DexValue::Long(value)));
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/atomic/AtomicReference;",
            "<init>",
            "()V",
        ) {
            let invocation = invocation?;
            self.initialize_empty_atomic_reference(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/atomic/AtomicReference;",
            "<init>",
            "(Ljava/lang/Object;)V",
        ) {
            let invocation = invocation?;
            self.initialize_atomic_reference(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/atomic/AtomicReference;",
            "get",
            "()Ljava/lang/Object;",
        ) {
            let invocation = invocation?;
            let value = self.atomic_reference_get(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/atomic/AtomicReference;",
            "set",
            "(Ljava/lang/Object;)V",
        ) {
            let invocation = invocation?;
            self.atomic_reference_set(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/atomic/AtomicReference;",
            "lazySet",
            "(Ljava/lang/Object;)V",
        ) {
            // cite: libcore AtomicReference.java#lazySet
            let invocation = invocation?;
            self.atomic_reference_set(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/atomic/AtomicReference;",
            "getAndSet",
            "(Ljava/lang/Object;)Ljava/lang/Object;",
        ) {
            let invocation = invocation?;
            let value = self.atomic_reference_get_and_set(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/atomic/AtomicReference;",
            "compareAndSet",
            "(Ljava/lang/Object;Ljava/lang/Object;)Z",
        ) {
            let invocation = invocation?;
            let value = self.atomic_reference_compare_and_set(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/atomic/AtomicReferenceArray;",
            "<init>",
            "(I)V",
        ) {
            let invocation = invocation?;
            self.initialize_atomic_reference_array(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/atomic/AtomicReferenceArray;",
            "length",
            "()I",
        ) {
            let invocation = invocation?;
            let value = self.atomic_reference_array_length(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/atomic/AtomicReferenceArray;",
            "get",
            "(I)Ljava/lang/Object;",
        ) {
            let invocation = invocation?;
            let value = self.atomic_reference_array_get(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/util/concurrent/atomic/AtomicReferenceArray;"
            && matches!(method.name.as_str(), "set" | "lazySet")
            && method.proto.descriptor_is("(ILjava/lang/Object;)V")
        {
            let invocation = invocation?;
            self.atomic_reference_array_set(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/atomic/AtomicReferenceArray;",
            "getAndSet",
            "(ILjava/lang/Object;)Ljava/lang/Object;",
        ) {
            let invocation = invocation?;
            let value = self.atomic_reference_array_get_and_set(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/atomic/AtomicReferenceArray;",
            "compareAndSet",
            "(ILjava/lang/Object;Ljava/lang/Object;)Z",
        ) {
            let invocation = invocation?;
            let value = self.atomic_reference_array_compare_and_set(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/atomic/AtomicReferenceFieldUpdater;",
            "newUpdater",
            "(Ljava/lang/Class;Ljava/lang/Class;Ljava/lang/String;)Ljava/util/concurrent/atomic/AtomicReferenceFieldUpdater;",
        )
        {
            let invocation = invocation?;
            let value = atomic_reference_field_updater_new(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/atomic/AtomicReferenceFieldUpdater;",
            "get",
            "(Ljava/lang/Object;)Ljava/lang/Object;",
        ) {
            let invocation = invocation?;
            let value = self.atomic_reference_field_updater_get(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        // cite: libcore AtomicReferenceFieldUpdater.java#lazySet
        if method.class_descriptor == "Ljava/util/concurrent/atomic/AtomicReferenceFieldUpdater;"
            && matches!(method.name.as_str(), "set" | "lazySet")
            && method
                .proto
                .descriptor_is("(Ljava/lang/Object;Ljava/lang/Object;)V")
        {
            let invocation = invocation?;
            self.atomic_reference_field_updater_set(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/atomic/AtomicReferenceFieldUpdater;",
            "getAndSet",
            "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
        ) {
            let invocation = invocation?;
            let value = self.atomic_reference_field_updater_get_and_set(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Ljava/util/concurrent/atomic/AtomicReferenceFieldUpdater;"
            && matches!(method.name.as_str(), "compareAndSet" | "weakCompareAndSet")
            && method.proto.descriptor()
                == "(Ljava/lang/Object;Ljava/lang/Object;Ljava/lang/Object;)Z"
        {
            let invocation = invocation?;
            let value = self.atomic_reference_field_updater_compare_and_set(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/atomic/AtomicIntegerFieldUpdater;",
            "newUpdater",
            "(Ljava/lang/Class;Ljava/lang/String;)Ljava/util/concurrent/atomic/AtomicIntegerFieldUpdater;",
        ) {
            let invocation = invocation?;
            let value = atomic_integer_field_updater_new(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/atomic/AtomicIntegerFieldUpdater;",
            "get",
            "(Ljava/lang/Object;)I",
        ) {
            let invocation = invocation?;
            let value = self.atomic_integer_field_updater_get(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.class_descriptor == "Ljava/util/concurrent/atomic/AtomicIntegerFieldUpdater;"
            && matches!(method.name.as_str(), "set" | "lazySet")
            && method.proto.descriptor_is("(Ljava/lang/Object;I)V")
        {
            let invocation = invocation?;
            self.atomic_integer_field_updater_set(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/util/concurrent/atomic/AtomicIntegerFieldUpdater;"
            && matches!(method.name.as_str(), "compareAndSet" | "weakCompareAndSet")
            && method.proto.descriptor_is("(Ljava/lang/Object;II)Z")
        {
            let invocation = invocation?;
            let value = self.atomic_integer_field_updater_compare_and_set(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/atomic/AtomicIntegerFieldUpdater;",
            "incrementAndGet",
            "(Ljava/lang/Object;)I",
        ) {
            let invocation = invocation?;
            let value = self.atomic_integer_field_updater_increment_and_get(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/atomic/AtomicIntegerFieldUpdater;",
            "getAndIncrement",
            "(Ljava/lang/Object;)I",
        ) {
            let invocation = invocation?;
            let value = self.atomic_integer_field_updater_get_and_increment(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/atomic/AtomicIntegerFieldUpdater;",
            "getAndDecrement",
            "(Ljava/lang/Object;)I",
        ) {
            let invocation = invocation?;
            let value = self.atomic_integer_field_updater_get_and_decrement(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/atomic/AtomicIntegerFieldUpdater;",
            "addAndGet",
            "(Ljava/lang/Object;I)I",
        ) {
            let invocation = invocation?;
            let value = self.atomic_integer_field_updater_add_and_get(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/atomic/AtomicIntegerFieldUpdater;",
            "decrementAndGet",
            "(Ljava/lang/Object;)I",
        ) {
            let invocation = invocation?;
            let value = self.atomic_integer_field_updater_decrement_and_get(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/atomic/AtomicLongFieldUpdater;",
            "newUpdater",
            "(Ljava/lang/Class;Ljava/lang/String;)Ljava/util/concurrent/atomic/AtomicLongFieldUpdater;",
        ) {
            let invocation = invocation?;
            let value = atomic_long_field_updater_new(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/atomic/AtomicLongFieldUpdater;",
            "get",
            "(Ljava/lang/Object;)J",
        ) {
            let invocation = invocation?;
            let value = self.atomic_long_field_updater_get(invocation)?;

            return Some(DexExecutionResult::Value(DexValue::Long(value)));
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/atomic/AtomicLongFieldUpdater;",
            "getAndIncrement",
            "(Ljava/lang/Object;)J",
        ) {
            let invocation = invocation?;
            let value = self.atomic_long_field_updater_get_and_increment(invocation)?;

            return Some(DexExecutionResult::Value(DexValue::Long(value)));
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/atomic/AtomicLongFieldUpdater;",
            "incrementAndGet",
            "(Ljava/lang/Object;)J",
        ) {
            let invocation = invocation?;
            let value = self.atomic_long_field_updater_increment_and_get(invocation)?;

            return Some(DexExecutionResult::Value(DexValue::Long(value)));
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/atomic/AtomicLongFieldUpdater;",
            "getAndDecrement",
            "(Ljava/lang/Object;)J",
        ) {
            let invocation = invocation?;
            let value = self.atomic_long_field_updater_get_and_decrement(invocation)?;

            return Some(DexExecutionResult::Value(DexValue::Long(value)));
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/atomic/AtomicLongFieldUpdater;",
            "decrementAndGet",
            "(Ljava/lang/Object;)J",
        ) {
            let invocation = invocation?;
            let value = self.atomic_long_field_updater_decrement_and_get(invocation)?;

            return Some(DexExecutionResult::Value(DexValue::Long(value)));
        }

        if method.matches_builtin_signature(
            "Ljava/util/concurrent/atomic/AtomicLongFieldUpdater;",
            "addAndGet",
            "(Ljava/lang/Object;J)J",
        ) {
            let invocation = invocation?;
            let value = self.atomic_long_field_updater_add_and_get(invocation)?;

            return Some(DexExecutionResult::Value(DexValue::Long(value)));
        }

        if method.class_descriptor == "Ljava/util/concurrent/atomic/AtomicLongFieldUpdater;"
            && matches!(method.name.as_str(), "set" | "lazySet")
            && method.proto.descriptor_is("(Ljava/lang/Object;J)V")
        {
            let invocation = invocation?;
            self.atomic_long_field_updater_set(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Ljava/util/concurrent/atomic/AtomicLongFieldUpdater;"
            && matches!(method.name.as_str(), "compareAndSet" | "weakCompareAndSet")
            && method.proto.descriptor_is("(Ljava/lang/Object;JJ)Z")
        {
            let invocation = invocation?;
            let value = self.atomic_long_field_updater_compare_and_set(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.matches_builtin_signature(
            "Lsun/misc/Unsafe;",
            "objectFieldOffset",
            "(Ljava/lang/reflect/Field;)J",
        ) {
            let invocation = invocation?;
            let value = self.unsafe_object_field_offset(invocation)?;

            return Some(DexExecutionResult::Value(DexValue::Long(value)));
        }

        if method.matches_builtin_signature(
            "Lsun/misc/Unsafe;",
            "compareAndSwapObject",
            "(Ljava/lang/Object;JLjava/lang/Object;Ljava/lang/Object;)Z",
        ) {
            let invocation = invocation?;
            let value = self.unsafe_compare_and_swap_object(invocation)?;

            return Some(DexExecutionResult::Int(value));
        }

        if method.matches_builtin_signature(
            "Lsun/misc/Unsafe;",
            "putObjectVolatile",
            "(Ljava/lang/Object;JLjava/lang/Object;)V",
        ) {
            let invocation = invocation?;
            self.unsafe_put_object_volatile(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.matches_builtin_signature(
            "Lsun/misc/Unsafe;",
            "getObjectVolatile",
            "(Ljava/lang/Object;J)Ljava/lang/Object;",
        ) {
            let invocation = invocation?;
            let value = self.unsafe_get_object_volatile(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.matches_builtin_signature(
            "Lsun/misc/Unsafe;",
            "getAndSetObject",
            "(Ljava/lang/Object;JLjava/lang/Object;)Ljava/lang/Object;",
        ) {
            let invocation = invocation?;
            let value = self.unsafe_get_and_set_object(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.matches_builtin_signature("Ljava/lang/ref/ReferenceQueue;", "<init>", "()V") {
            return Some(DexExecutionResult::Void);
        }

        if method.matches_builtin_signature(
            "Ljava/lang/ref/ReferenceQueue;",
            "poll",
            "()Ljava/lang/ref/Reference;",
        ) {
            return Some(DexExecutionResult::Value(DexValue::Null));
        }

        if matches!(
            method.class_descriptor.as_str(),
            "Ljava/lang/ref/WeakReference;" | "Ljava/lang/ref/SoftReference;"
        ) && method.name == "<init>"
            && method.proto.descriptor_is("(Ljava/lang/Object;)V")
        {
            let invocation = invocation?;
            self.initialize_weak_reference(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if matches!(
            method.class_descriptor.as_str(),
            "Ljava/lang/ref/WeakReference;" | "Ljava/lang/ref/SoftReference;"
        ) && method.name == "<init>"
            && method
                .proto
                .descriptor_is("(Ljava/lang/Object;Ljava/lang/ref/ReferenceQueue;)V")
        {
            let invocation = invocation?;
            self.initialize_weak_reference_with_queue(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        // cite: libcore Reference.java#clear
        if matches!(
            method.class_descriptor.as_str(),
            "Ljava/lang/ref/Reference;"
                | "Ljava/lang/ref/WeakReference;"
                | "Ljava/lang/ref/SoftReference;"
        ) && method.name == "clear"
            && method.proto.descriptor_is("()V")
        {
            let invocation = invocation?;
            self.weak_reference_clear(invocation)?;

            return Some(DexExecutionResult::Void);
        }

        if method.matches_builtin_signature(
            "Ljava/lang/ref/WeakReference;",
            "get",
            "()Ljava/lang/Object;",
        ) {
            let value = self.weak_reference_get(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.matches_builtin_signature(
            "Ljava/lang/ref/Reference;",
            "get",
            "()Ljava/lang/Object;",
        ) {
            let value = self.weak_reference_get(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        None
    }
}

fn lock_support_park_nanos(invocation: &DexInvocation) -> Option<bool> {
    if invocation.arguments.len() != 2 {
        return None;
    }

    let DexValue::Long(nanos) = invocation.arguments[0] else {
        return None;
    };
    let DexValue::Long(high_slot_nanos) = invocation.arguments[1] else {
        return None;
    };
    if nanos != high_slot_nanos {
        return None;
    }

    Some(nanos > 0)
}

fn lock_support_park_nanos_with_blocker(invocation: &DexInvocation) -> Option<bool> {
    if invocation.arguments.len() != 3 {
        return None;
    }

    if !dex_value_is_nullable_object_like(&invocation.arguments[0]) {
        return None;
    }

    let DexValue::Long(nanos) = invocation.arguments[1] else {
        return None;
    };
    let DexValue::Long(high_slot_nanos) = invocation.arguments[2] else {
        return None;
    };
    if nanos != high_slot_nanos {
        return None;
    }

    Some(nanos > 0)
}

impl DexProgram {
    fn lock_support_unpark(&self, invocation: &DexInvocation) -> Option<()> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        match &invocation.arguments[0] {
            DexValue::Null | DexValue::Int(0) => Some(()),
            value if dex_value_is_object_like(value) => {
                self.runtime.borrow_mut().unpark_thread(value)?;
                Some(())
            }
            _ => None,
        }
    }
}

fn is_builtin_lock_class(class_descriptor: &str) -> bool {
    matches!(
        class_descriptor,
        "Ljava/util/concurrent/locks/Lock;"
            | "Ljava/util/concurrent/locks/ReentrantLock;"
            | "Ljava/util/concurrent/locks/ReentrantReadWriteLock$ReadLock;"
            | "Ljava/util/concurrent/locks/ReentrantReadWriteLock$WriteLock;"
    )
}

fn reentrant_read_write_lock_accessor(name: &str) -> Option<(DexFieldId, &'static str)> {
    let (field_name, class_descriptor) = match name {
        "readLock" => (
            "readerLock",
            "Ljava/util/concurrent/locks/ReentrantReadWriteLock$ReadLock;",
        ),
        "writeLock" => (
            "writerLock",
            "Ljava/util/concurrent/locks/ReentrantReadWriteLock$WriteLock;",
        ),
        _ => return None,
    };

    Some((
        DexFieldId {
            class_descriptor: "Ljava/util/concurrent/locks/ReentrantReadWriteLock;".to_owned(),
            type_descriptor: class_descriptor.to_owned(),
            name: field_name.to_owned(),
        },
        class_descriptor,
    ))
}

impl DexProgram {
    // cite: libcore ReentrantReadWriteLock.java#ReentrantReadWriteLock, readerLock, writerLock
    fn reentrant_read_write_lock_init(&self, invocation: &DexInvocation) -> Option<()> {
        let lock = invocation.arguments.first()?;
        if !dex_value_is_object_like(lock) {
            return None;
        }
        for name in ["readLock", "writeLock"] {
            let (field, class_descriptor) = reentrant_read_write_lock_accessor(name)?;
            let child = self.allocate_object(class_descriptor.to_owned());
            if !self.write_stored_instance_field(lock, &field, child) {
                return None;
            }
        }

        Some(())
    }
}

impl DexProgram {
    fn condition_signal(&self, invocation: &DexInvocation, all: bool) -> Option<()> {
        if invocation.arguments.len() != 1 {
            return None;
        }
        let condition_key = map_identity_key(&invocation.arguments[0]);
        let mut runtime = self.runtime.borrow_mut();
        let mut waiters = runtime
            .condition_waiters
            .remove(&condition_key)
            .unwrap_or_default();
        let wake = if all {
            std::mem::take(&mut waiters)
        } else if waiters.is_empty() {
            Vec::new()
        } else {
            vec![waiters.remove(0)]
        };
        if !waiters.is_empty() {
            runtime.condition_waiters.insert(condition_key, waiters);
        }
        for thread in wake {
            runtime.unpark_thread(&thread);
        }

        Some(())
    }
}
