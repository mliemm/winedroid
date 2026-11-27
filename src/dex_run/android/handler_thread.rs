use super::handler_message::looper_thread_field;
use super::*;

const HANDLER_THREAD: &str = "Landroid/os/HandlerThread;";
const LOOPER: &str = "Landroid/os/Looper;";

fn handler_thread_looper_field() -> DexFieldId {
    DexFieldId {
        class_descriptor: HANDLER_THREAD.to_owned(),
        type_descriptor: LOOPER.to_owned(),
        name: "mLooper".to_owned(),
    }
}

fn handler_thread_tid_field() -> DexFieldId {
    DexFieldId {
        class_descriptor: HANDLER_THREAD.to_owned(),
        type_descriptor: "I".to_owned(),
        name: "mTid".to_owned(),
    }
}

impl DexProgram {
    // cite: frameworks-base Looper.java#myLooper, sThreadLocal, prepare
    pub(in crate::dex_run) fn current_thread_looper(&self) -> DexValue {
        let thread = self.runtime.borrow().current_guest_thread.clone();
        if let Some(looper) = self
            .runtime
            .borrow()
            .thread_loopers
            .get(&map_identity_key(&thread))
            .cloned()
        {
            return looper;
        }
        if self.current_guest_thread_is_main() {
            return DexValue::Reference(std::rc::Rc::from("Landroid/os/Looper;:main"));
        }
        match self.read_stored_instance_field(&thread, &handler_thread_looper_field()) {
            Some(looper @ (DexValue::Object(_) | DexValue::Reference(_))) => looper,
            _ => DexValue::Null,
        }
    }

    pub(in crate::dex_run) fn set_current_thread_looper(&self, looper: Option<DexValue>) {
        let thread = map_identity_key(&self.runtime.borrow().current_guest_thread);
        let mut runtime = self.runtime.borrow_mut();
        match looper {
            Some(looper) => {
                runtime.thread_loopers.insert(thread, looper);
            }
            None => {
                runtime.thread_loopers.remove(&thread);
            }
        }
    }

    // cite: frameworks-base Process.java#myTid
    pub(in crate::dex_run) fn current_guest_thread_tid(&self) -> i32 {
        if self.current_guest_thread_is_main() {
            return BOOTSTRAP_PROCESS_ID;
        }
        let mut runtime = self.runtime.borrow_mut();
        let thread = map_identity_key(&runtime.current_guest_thread);
        let next = BOOTSTRAP_PROCESS_ID + 1 + runtime.guest_thread_ids.len() as i32;
        *runtime.guest_thread_ids.entry(thread).or_insert(next)
    }

    fn known_guest_tid(&self, tid: i32) -> bool {
        tid == BOOTSTRAP_PROCESS_ID
            || self
                .runtime
                .borrow()
                .guest_thread_ids
                .values()
                .any(|known_tid| *known_tid == tid)
    }

    // cite: frameworks-base Process.java#setThreadPriority
    pub(in crate::dex_run) fn process_set_thread_priority(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexExecutionResult> {
        let (tid, priority) = match invocation.arguments.as_slice() {
            [DexValue::Int(0), DexValue::Int(priority)] => {
                (self.current_guest_thread_tid(), *priority)
            }
            [DexValue::Int(tid), DexValue::Int(priority)] => (*tid, *priority),
            [DexValue::Int(priority)] => (self.current_guest_thread_tid(), *priority),
            _ => return None,
        };
        if !self.known_guest_tid(tid) {
            return Some(DexExecutionResult::Throw(
                self.allocate_object("Ljava/lang/IllegalArgumentException;".to_owned()),
            ));
        }
        self.runtime
            .borrow_mut()
            .thread_priorities
            .insert(tid, priority);

        Some(DexExecutionResult::Void)
    }

    // cite: frameworks-base Process.java#getThreadPriority, THREAD_PRIORITY_DEFAULT
    pub(in crate::dex_run) fn process_get_thread_priority(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexExecutionResult> {
        let [DexValue::Int(tid)] = invocation.arguments.as_slice() else {
            return None;
        };
        let tid = if *tid == 0 {
            self.current_guest_thread_tid()
        } else {
            *tid
        };
        if !self.known_guest_tid(tid) {
            return Some(DexExecutionResult::Throw(
                self.allocate_object("Ljava/lang/IllegalArgumentException;".to_owned()),
            ));
        }
        let priority = self
            .runtime
            .borrow()
            .thread_priorities
            .get(&tid)
            .copied()
            .unwrap_or(0);

        Some(DexExecutionResult::Int(priority))
    }

    // cite: frameworks-base HandlerThread.java#getThreadId, mTid
    pub(in crate::dex_run) fn handler_thread_thread_id(&self, thread: &DexValue) -> Option<i32> {
        if !dex_value_is_object_like(thread) {
            return None;
        }
        match self.read_stored_instance_field(thread, &handler_thread_tid_field()) {
            Some(DexValue::Int(tid)) => Some(tid),
            _ => Some(-1),
        }
    }

    // cite: frameworks-base HandlerThread.java#run, onLooperPrepared, mLooper, mTid
    // cite: frameworks-base Looper.java#prepare, mThread
    pub(in crate::dex_run) fn run_handler_thread(
        &self,
        thread: &DexValue,
        depth: usize,
    ) -> Result<()> {
        let tid = self.current_guest_thread_tid();
        self.write_stored_instance_field(thread, &handler_thread_tid_field(), DexValue::Int(tid));
        // cite: frameworks-base HandlerThread.java#run, mPriority
        let priority_field = DexFieldId {
            class_descriptor: "Landroid/os/HandlerThread;".to_owned(),
            type_descriptor: "I".to_owned(),
            name: "mPriority".to_owned(),
        };
        if let Some(DexValue::Int(priority)) =
            self.read_stored_instance_field(thread, &priority_field)
        {
            self.runtime
                .borrow_mut()
                .thread_priorities
                .insert(tid, priority);
        }
        let looper = self.allocate_object(LOOPER.to_owned());
        if !self.write_stored_instance_field(thread, &handler_thread_looper_field(), looper.clone())
        {
            return Err(WinedroidError::new(format!(
                "HandlerThread.run requires an object-backed receiver\nreceiver: {thread}"
            )));
        }
        self.write_stored_instance_field(&looper, &looper_thread_field(), thread.clone());

        let Some(class_descriptor) = value_class_descriptor(thread) else {
            return Ok(());
        };
        let on_looper_prepared = DexMethodId {
            class_descriptor: class_descriptor.to_owned(),
            name: "onLooperPrepared".to_owned(),
            proto: dex::DexProto {
                shorty: "V".to_owned(),
                return_type: "V".to_owned(),
                parameters: Vec::new(),
            },
        };
        let Some(resolved) =
            self.resolve_exact_method_in_class_hierarchy(class_descriptor, &on_looper_prepared)?
        else {
            return if self.park_handler_thread(thread) {
                Ok(())
            } else {
                Err(WinedroidError::new(format!(
                    "HandlerThread.run could not park the looper thread\nclass: {class_descriptor}"
                )))
            };
        };
        match self.execute_method(None, &resolved, Some(&[thread.clone()]), depth + 1)? {
            DexExecutionResult::Void => {
                if self.park_handler_thread(thread) {
                    Ok(())
                } else {
                    Err(WinedroidError::new(format!(
                        "HandlerThread.run could not park the looper thread\nclass: {class_descriptor}"
                    )))
                }
            }
            DexExecutionResult::Parked(execution) => {
                let cleanup_execution = execution.clone();
                if self
                    .runtime
                    .borrow_mut()
                    .store_parked_thread(thread, execution)
                    .is_some()
                {
                    Ok(())
                } else {
                    self.discard_parked_execution(&cleanup_execution);
                    Err(WinedroidError::new(format!(
                        "HandlerThread.run could not store the parked looper thread\nclass: {class_descriptor}"
                    )))
                }
            }
            _ => Err(WinedroidError::new(format!(
                "HandlerThread.onLooperPrepared did not complete normally\nclass: {class_descriptor}"
            ))),
        }
    }

    // cite: frameworks-base HandlerThread.java#run
    // cite: frameworks-base Looper.java#loop
    fn park_handler_thread(&self, thread: &DexValue) -> bool {
        self.runtime
            .borrow_mut()
            .store_parked_thread(thread, DexParkedExecution::empty())
            .is_some()
    }

    // cite: frameworks-base HandlerThread.java#getLooper
    pub(in crate::dex_run) fn handler_thread_looper(
        &self,
        invocation: &DexInvocation,
        depth: usize,
    ) -> Result<DexValue> {
        let [thread] = invocation.arguments.as_slice() else {
            return Err(WinedroidError::new(format!(
                "HandlerThread.getLooper expected receiver only, got {} arguments",
                invocation.arguments.len()
            )));
        };
        if !dex_value_is_object_like(thread) {
            return Err(WinedroidError::new(format!(
                "HandlerThread.getLooper requires an object-backed receiver\nreceiver: {thread}"
            )));
        }

        loop {
            if !self.thread_is_alive(thread) {
                return Ok(DexValue::Null);
            }
            match self.read_stored_instance_field(thread, &handler_thread_looper_field()) {
                Some(looper) if looper != DexValue::Null => return Ok(looper),
                _ => {}
            }
            if self.run_pending_guest_threads(depth)? == 0 {
                return Err(WinedroidError::new(format!(
                    "HandlerThread.getLooper would block forever: no runnable guest thread\nreceiver: {thread}"
                )));
            }
        }
    }
}
