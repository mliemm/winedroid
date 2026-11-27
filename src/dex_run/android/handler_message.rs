use super::*;

const HANDLER: &str = "Landroid/os/Handler;";
const MESSAGE: &str = "Landroid/os/Message;";
const LOOPER: &str = "Landroid/os/Looper;";
const FLAG_IN_USE: i32 = 1;

fn message_flags_field() -> DexFieldId {
    message_int_field("flags")
}

pub(in crate::dex_run) fn message_callback_field() -> DexFieldId {
    DexFieldId {
        class_descriptor: MESSAGE.to_owned(),
        type_descriptor: "Ljava/lang/Runnable;".to_owned(),
        name: "callback".to_owned(),
    }
}

fn message_typed_field(name: &str, type_descriptor: &str) -> DexFieldId {
    DexFieldId {
        class_descriptor: MESSAGE.to_owned(),
        type_descriptor: type_descriptor.to_owned(),
        name: name.to_owned(),
    }
}

fn wide_long_argument(value: &DexValue, high_slot: &DexValue) -> Option<i64> {
    match (value, high_slot) {
        (DexValue::Long(value), DexValue::Long(high_slot)) if value == high_slot => Some(*value),
        _ => None,
    }
}

fn handler_callback_field() -> DexFieldId {
    DexFieldId {
        class_descriptor: HANDLER.to_owned(),
        type_descriptor: "Landroid/os/Handler$Callback;".to_owned(),
        name: "mCallback".to_owned(),
    }
}

fn looper_logging_field() -> DexFieldId {
    DexFieldId {
        class_descriptor: LOOPER.to_owned(),
        type_descriptor: "Landroid/util/Printer;".to_owned(),
        name: "mLogging".to_owned(),
    }
}

impl DexProgram {
    // cite: frameworks-base Looper.java#setMessageLogging, mLogging
    pub(in crate::dex_run) fn looper_set_message_logging(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        let [looper, printer] = invocation.arguments.as_slice() else {
            return None;
        };
        let printer = if is_android_null_value(printer) {
            DexValue::Null
        } else if dex_value_is_object_like(printer) {
            printer.clone()
        } else {
            return None;
        };
        self.write_stored_instance_field(looper, &looper_logging_field(), printer)
            .then_some(())
    }

    pub(in crate::dex_run) fn dispatch_handler_post(
        &self,
        post: &HandlerPost,
        depth: usize,
    ) -> Result<()> {
        let dispatch = || self.dispatch_runnable(&post.runnable, "Handler.post", depth);
        match &post.handler {
            Some(handler) => {
                self.with_looper_logging(handler, Some(&post.runnable), 0, depth, dispatch)
            }
            None => dispatch(),
        }
    }

    // cite: frameworks-base Looper.java#loopOnce, mLogging
    fn with_looper_logging(
        &self,
        handler: &DexValue,
        callback: Option<&DexValue>,
        what: i32,
        depth: usize,
        dispatch: impl FnOnce() -> Result<()>,
    ) -> Result<()> {
        let key = map_identity_key(handler);
        let looper = self.runtime.borrow().handler_loopers.get(&key).cloned();
        let printer = looper
            .and_then(|looper| self.read_stored_instance_field(&looper, &looper_logging_field()));
        let Some(printer @ (DexValue::Object(_) | DexValue::Reference(_))) = printer else {
            return dispatch();
        };

        let target = self.looper_dispatch_value_text(handler, true, depth)?;
        let callback_start = match callback {
            Some(callback) => self.looper_dispatch_value_text(callback, false, depth)?,
            None => "null".to_owned(),
        };
        self.looper_print(
            &printer,
            format!(">>>>> Dispatching to {target} {callback_start}: {what}"),
            depth,
        )?;
        dispatch()?;
        let target = self.looper_dispatch_value_text(handler, true, depth)?;
        let callback_end = match callback {
            Some(callback) => self.looper_dispatch_value_text(callback, false, depth)?,
            None => "null".to_owned(),
        };
        self.looper_print(
            &printer,
            format!("<<<<< Finished to {target} {callback_end}"),
            depth,
        )
    }

    // cite: frameworks-base Handler.java#toString
    fn looper_dispatch_value_text(
        &self,
        value: &DexValue,
        handler: bool,
        depth: usize,
    ) -> Result<String> {
        let Some(descriptor) = value_class_descriptor(value) else {
            return Err(WinedroidError::new(
                "Looper logging requires an object-backed dispatch target",
            ));
        };
        let method = DexMethodId {
            class_descriptor: descriptor.to_owned(),
            name: "toString".to_owned(),
            proto: dex::DexProto {
                shorty: "L".to_owned(),
                return_type: "Ljava/lang/String;".to_owned(),
                parameters: Vec::new(),
            },
        };
        if self
            .resolve_exact_method_in_class_hierarchy(descriptor, &method)?
            .is_some()
        {
            let value = self.dispatch_object_value_method(
                value,
                &method.name,
                method.proto,
                Vec::new(),
                "Looper logging",
                depth,
            )?;
            if is_android_null_value(&value) {
                return Ok("null".to_owned());
            }
            return self.char_sequence_text(&value).ok_or_else(|| {
                WinedroidError::new("Looper logging toString did not return a string")
            });
        }
        if handler {
            let class =
                descriptor_to_java_name(descriptor).unwrap_or_else(|| descriptor.to_owned());
            return Ok(format!(
                "Handler ({class}) {{{:x}}}",
                stable_hash_i32(&map_identity_key(value)) as u32
            ));
        }
        self.object_value_to_string(value)
            .ok_or_else(|| WinedroidError::new("Looper logging could not format a callback"))
    }

    // cite: frameworks-base Printer.java#println
    fn looper_print(&self, printer: &DexValue, text: String, depth: usize) -> Result<()> {
        self.dispatch_object_void_method(
            printer,
            "println",
            dex::DexProto {
                shorty: "VL".to_owned(),
                return_type: "V".to_owned(),
                parameters: vec!["Ljava/lang/String;".to_owned()],
            },
            vec![DexValue::String(text.into())],
            "Looper message logging",
            depth,
        )
    }

    // cite: frameworks-base Handler.java#sendMessage, sendMessageDelayed
    pub(in crate::dex_run) fn handler_send_message(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexExecutionResult> {
        let [handler, message] = invocation.arguments.as_slice() else {
            return None;
        };

        self.handler_enqueue_message(handler, message, 0, false)
    }

    // cite: frameworks-base Handler.java#sendMessageAtFrontOfQueue
    pub(in crate::dex_run) fn handler_send_message_at_front_of_queue(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexExecutionResult> {
        let [handler, message] = invocation.arguments.as_slice() else {
            return None;
        };

        self.handler_enqueue_message(handler, message, 0, true)
    }

    // cite: frameworks-base Handler.java#sendMessageAtTime
    // cite: frameworks-base MessageQueue.java#enqueueMessage
    pub(in crate::dex_run) fn handler_send_message_at_time(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexExecutionResult> {
        let [handler, message, uptime_millis, high_slot] = invocation.arguments.as_slice() else {
            return None;
        };
        let uptime_millis = wide_long_argument(uptime_millis, high_slot)?;
        let now_millis = self.runtime.borrow().main_thread_uptime_millis;
        let delay_millis = uptime_millis.saturating_sub(now_millis);

        self.handler_enqueue_message(handler, message, delay_millis, false)
    }

    // cite: frameworks-base Handler.java#sendMessageDelayed
    pub(in crate::dex_run) fn handler_send_message_delayed(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexExecutionResult> {
        let [handler, message, delay_millis, high_slot] = invocation.arguments.as_slice() else {
            return None;
        };
        let delay_millis = wide_long_argument(delay_millis, high_slot)?;

        self.handler_enqueue_message(handler, message, delay_millis, false)
    }

    // cite: frameworks-base Handler.java#sendEmptyMessageDelayed
    // cite: frameworks-base Message.java#obtain
    pub(in crate::dex_run) fn handler_send_empty_message_delayed(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexExecutionResult> {
        let [handler, DexValue::Int(what), delay_millis, high_slot] =
            invocation.arguments.as_slice()
        else {
            return None;
        };
        let delay_millis = wide_long_argument(delay_millis, high_slot)?;
        if !self.handler_has_looper(handler) {
            return None;
        }

        let message = self.allocate_object(MESSAGE.to_owned());
        self.write_stored_instance_field(
            &message,
            &message_int_field("what"),
            DexValue::Int(*what),
        );

        self.handler_enqueue_message(handler, &message, delay_millis, false)
    }

    fn handler_has_looper(&self, handler: &DexValue) -> bool {
        dex_value_is_object_like(handler)
            && self
                .runtime
                .borrow()
                .handler_loopers
                .contains_key(&map_identity_key(handler))
    }

    // cite: frameworks-base Handler.java#sendMessageDelayed, sendMessageAtTime, enqueueMessage
    // cite: frameworks-base MessageQueue.java#enqueueMessage
    fn handler_enqueue_message(
        &self,
        handler: &DexValue,
        message: &DexValue,
        delay_millis: i64,
        at_front: bool,
    ) -> Option<DexExecutionResult> {
        if !self.handler_has_looper(handler) {
            return None;
        }
        if !dex_value_is_object_like(message) {
            return Some(DexExecutionResult::Throw(
                self.allocate_object("Ljava/lang/NullPointerException;".to_owned()),
            ));
        }

        self.write_stored_instance_field(
            message,
            &message_handler_field("target"),
            handler.clone(),
        );
        let flags = match self.read_stored_instance_field(message, &message_flags_field()) {
            Some(DexValue::Int(flags)) => flags,
            _ => 0,
        };
        if flags & FLAG_IN_USE != 0 {
            return Some(DexExecutionResult::Throw(
                self.allocate_object("Ljava/lang/IllegalStateException;".to_owned()),
            ));
        }
        self.write_stored_instance_field(
            message,
            &message_flags_field(),
            DexValue::Int(flags | FLAG_IN_USE),
        );
        self.runtime.borrow_mut().post_handler_runnable(
            map_identity_key(handler),
            message.clone(),
            delay_millis.max(0),
            at_front,
        );

        Some(DexExecutionResult::Int(1))
    }

    fn handler_message_query(
        &self,
        invocation: &DexInvocation,
    ) -> Option<(String, i32, Option<String>)> {
        let (handler, what, object) = match invocation.arguments.as_slice() {
            [handler, DexValue::Int(what)] => (handler, *what, &DexValue::Null),
            [handler, DexValue::Int(what), object] => (handler, *what, object),
            _ => return None,
        };
        if !self.handler_has_looper(handler) {
            return None;
        }
        let object_key = match object {
            DexValue::Null | DexValue::Int(0) => None,
            DexValue::Object(_) | DexValue::Reference(_) => Some(map_identity_key(object)),
            _ => return None,
        };

        Some((map_identity_key(handler), what, object_key))
    }

    fn queued_post_matches(
        &self,
        post: &HandlerPost,
        handler_key: &str,
        what: i32,
        object_key: Option<&String>,
    ) -> bool {
        if post.handler_key != handler_key {
            return false;
        }
        let (post_what, post_object) = self.queued_post_what_and_obj(&post.runnable);

        post_what == what && object_key.is_none_or(|key| post_object.as_ref() == Some(key))
    }

    // cite: frameworks-base Handler.java#hasMessages
    // cite: frameworks-base MessageQueue.java#hasMessages
    pub(in crate::dex_run) fn handler_has_messages(
        &self,
        invocation: &DexInvocation,
    ) -> Option<i32> {
        let (handler_key, what, object_key) = self.handler_message_query(invocation)?;
        let posts = self.runtime.borrow().handler_posts.clone();

        Some(i32::from(posts.iter().any(|post| {
            self.queued_post_matches(post, &handler_key, what, object_key.as_ref())
        })))
    }

    // cite: frameworks-base Handler.java#removeMessages, getPostMessage
    // cite: frameworks-base MessageQueue.java#removeMessages
    pub(in crate::dex_run) fn handler_remove_messages(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        let (handler_key, what, object_key) = self.handler_message_query(invocation)?;
        let posts = std::mem::take(&mut self.runtime.borrow_mut().handler_posts);
        let (removed, kept): (Vec<_>, Vec<_>) = posts.into_iter().partition(|post| {
            self.queued_post_matches(post, &handler_key, what, object_key.as_ref())
        });
        self.runtime.borrow_mut().handler_posts = kept;
        for post in removed {
            if value_class_descriptor(&post.runnable) == Some(MESSAGE) {
                self.recycle_message(&post.runnable);
            }
        }

        Some(())
    }

    fn queued_post_what_and_obj(&self, queued: &DexValue) -> (i32, Option<String>) {
        if value_class_descriptor(queued) != Some(MESSAGE) {
            return (0, None);
        }
        let what = match self.read_stored_instance_field(queued, &message_int_field("what")) {
            Some(DexValue::Int(what)) => what,
            _ => 0,
        };
        let object = match self.read_stored_instance_field(queued, &message_object_field("obj")) {
            Some(object @ (DexValue::Object(_) | DexValue::Reference(_))) => {
                Some(map_identity_key(&object))
            }
            _ => None,
        };

        (what, object)
    }

    // cite: frameworks-base Looper.java#loopOnce
    // cite: frameworks-base Handler.java#dispatchMessage, handleCallback
    // cite: frameworks-base Message.java#recycleUnchecked
    pub(in crate::dex_run) fn dispatch_handler_message(
        &self,
        message: &DexValue,
        depth: usize,
    ) -> Result<()> {
        let Some(handler @ (DexValue::Object(_) | DexValue::Reference(_))) =
            self.read_stored_instance_field(message, &message_handler_field("target"))
        else {
            return Err(WinedroidError::new(format!(
                "Handler message dispatch requires a message with a target\nmessage: {message}"
            )));
        };

        let message_callback = self
            .read_stored_instance_field(message, &message_callback_field())
            .filter(dex_value_is_object_like);
        let what = match self.read_stored_instance_field(message, &message_int_field("what")) {
            Some(DexValue::Int(what)) => what,
            _ => 0,
        };
        self.with_looper_logging(&handler, message_callback.as_ref(), what, depth, || {
            if let Some(callback) = &message_callback {
                self.dispatch_runnable(callback, "Message.callback", depth)?;
            } else if !self.dispatch_handler_callback(&handler, message, depth)? {
                self.dispatch_handler_handle_message(&handler, message, depth)?;
            }
            Ok(())
        })?;

        self.recycle_message(message);

        Ok(())
    }

    fn dispatch_handler_callback(
        &self,
        handler: &DexValue,
        message: &DexValue,
        depth: usize,
    ) -> Result<bool> {
        let Some(callback @ (DexValue::Object(_) | DexValue::Reference(_))) =
            self.read_stored_instance_field(handler, &handler_callback_field())
        else {
            return Ok(false);
        };
        let Some(class_descriptor) = value_class_descriptor(&callback) else {
            return Ok(false);
        };
        let handle_message = DexMethodId {
            class_descriptor: class_descriptor.to_owned(),
            name: "handleMessage".to_owned(),
            proto: dex::DexProto {
                shorty: "ZL".to_owned(),
                return_type: "Z".to_owned(),
                parameters: vec![MESSAGE.to_owned()],
            },
        };
        let Some(resolved) =
            self.resolve_exact_method_in_class_hierarchy(class_descriptor, &handle_message)?
        else {
            return Err(WinedroidError::new(format!(
                "Handler.Callback.handleMessage was not found\ncallback class: {class_descriptor}"
            )));
        };
        match self.execute_method(
            None,
            &resolved,
            Some(&[callback.clone(), message.clone()]),
            depth + 1,
        )? {
            DexExecutionResult::Int(handled) => Ok(handled != 0),
            _ => Err(WinedroidError::new(format!(
                "Handler.Callback.handleMessage did not return normally\ncallback class: {class_descriptor}"
            ))),
        }
    }

    fn dispatch_handler_handle_message(
        &self,
        handler: &DexValue,
        message: &DexValue,
        depth: usize,
    ) -> Result<()> {
        let Some(class_descriptor) = value_class_descriptor(handler) else {
            return Ok(());
        };
        let handle_message = DexMethodId {
            class_descriptor: class_descriptor.to_owned(),
            name: "handleMessage".to_owned(),
            proto: dex::DexProto {
                shorty: "VL".to_owned(),
                return_type: "V".to_owned(),
                parameters: vec![MESSAGE.to_owned()],
            },
        };
        let Some(resolved) =
            self.resolve_exact_method_in_class_hierarchy(class_descriptor, &handle_message)?
        else {
            return Ok(());
        };
        match self.execute_method(
            None,
            &resolved,
            Some(&[handler.clone(), message.clone()]),
            depth + 1,
        )? {
            DexExecutionResult::Void => Ok(()),
            _ => Err(WinedroidError::new(format!(
                "Handler.handleMessage did not complete normally\nhandler class: {class_descriptor}"
            ))),
        }
    }

    fn recycle_message(&self, message: &DexValue) {
        self.write_stored_instance_field(
            message,
            &message_flags_field(),
            DexValue::Int(FLAG_IN_USE),
        );
        self.write_stored_instance_field(message, &message_int_field("what"), DexValue::Int(0));
        self.write_stored_instance_field(message, &message_int_field("arg1"), DexValue::Int(0));
        self.write_stored_instance_field(message, &message_int_field("arg2"), DexValue::Int(0));
        self.write_stored_instance_field(
            message,
            &message_typed_field("replyTo", "Landroid/os/Messenger;"),
            DexValue::Null,
        );
        self.write_stored_instance_field(
            message,
            &message_typed_field("data", "Landroid/os/Bundle;"),
            DexValue::Null,
        );
        self.write_stored_instance_field(message, &message_object_field("obj"), DexValue::Null);
        self.write_stored_instance_field(message, &message_handler_field("target"), DexValue::Null);
        self.write_stored_instance_field(message, &message_callback_field(), DexValue::Null);
    }
}

impl DexProgram {
    // cite: frameworks-base Looper.java#getThread, mThread
    pub(in crate::dex_run) fn handler_looper_thread(&self, handler_key: &str) -> Option<DexValue> {
        let looper = self
            .runtime
            .borrow()
            .handler_loopers
            .get(handler_key)
            .cloned()?;
        match self.read_stored_instance_field(&looper, &looper_thread_field())? {
            thread @ (DexValue::Object(_) | DexValue::Reference(_)) => Some(thread),
            _ => None,
        }
    }
}

pub(in crate::dex_run) fn looper_thread_field() -> DexFieldId {
    DexFieldId {
        class_descriptor: "Landroid/os/Looper;".to_owned(),
        type_descriptor: "Ljava/lang/Thread;".to_owned(),
        name: "mThread".to_owned(),
    }
}

fn looper_queue_field() -> DexFieldId {
    DexFieldId {
        class_descriptor: LOOPER.to_owned(),
        type_descriptor: "Landroid/os/MessageQueue;".to_owned(),
        name: "mQueue".to_owned(),
    }
}

impl DexProgram {
    // cite: frameworks-base MessageQueue.java#addIdleHandler, removeIdleHandler, mIdleHandlers
    pub(in crate::dex_run) fn message_queue_idle_handler(
        &self,
        invocation: &DexInvocation,
        add: bool,
    ) -> Option<DexExecutionResult> {
        let [queue, handler] = invocation.arguments.as_slice() else {
            return None;
        };
        if !dex_value_is_object_like(queue) {
            return None;
        }
        if is_android_null_value(handler) {
            return Some(if add {
                self.null_pointer_exception_result()
            } else {
                DexExecutionResult::Void
            });
        }
        let entry = (map_identity_key(queue), handler.clone());
        let mut runtime = self.runtime.borrow_mut();
        if add {
            runtime.idle_handlers.push(entry);
        } else if let Some(index) = runtime
            .idle_handlers
            .iter()
            .position(|known| *known == entry)
        {
            runtime.idle_handlers.remove(index);
        }

        Some(DexExecutionResult::Void)
    }

    // cite: frameworks-base MessageQueue.java#next, queueIdle, pendingIdleHandlerCount
    pub(in crate::dex_run) fn run_main_idle_handlers(&self, depth: usize) -> Result<bool> {
        let main_looper = DexValue::Reference(std::rc::Rc::from("Landroid/os/Looper;:main"));
        let Some(queue) = self.read_stored_instance_field(&main_looper, &looper_queue_field())
        else {
            return Ok(false);
        };
        let queue_key = map_identity_key(&queue);
        let handlers: Vec<DexValue> = self
            .runtime
            .borrow()
            .idle_handlers
            .iter()
            .filter(|(owner, _)| *owner == queue_key)
            .map(|(_, handler)| handler.clone())
            .collect();
        if handlers.is_empty() {
            return Ok(false);
        }

        let queue_idle = DexMethodId {
            class_descriptor: "Landroid/os/MessageQueue$IdleHandler;".to_owned(),
            name: "queueIdle".to_owned(),
            proto: crate::dex::DexProto {
                shorty: "Z".to_owned(),
                return_type: "Z".to_owned(),
                parameters: Vec::new(),
            },
        };
        for handler in handlers {
            let Some(class_descriptor) = value_class_descriptor(&handler) else {
                continue;
            };
            let receiver_method = DexMethodId {
                class_descriptor: class_descriptor.to_owned(),
                ..queue_idle.clone()
            };
            let Some(resolved) =
                self.resolve_exact_method_in_class_hierarchy(class_descriptor, &receiver_method)?
            else {
                return Err(WinedroidError::new(format!(
                    "MessageQueue.IdleHandler.queueIdle was not found\nhandler: {handler}"
                )));
            };
            let keep = match self.execute_method(
                None,
                &resolved,
                Some(&[handler.clone()]),
                depth + 1,
            )? {
                DexExecutionResult::Int(keep) => keep != 0,
                DexExecutionResult::Parked(execution) => {
                    self.discard_parked_execution(&execution);
                    return Err(WinedroidError::new(format!(
                        "MessageQueue.IdleHandler.queueIdle parked instead of returning\nhandler: {handler}\nnext missing primitive: blocking wait inside an idle handler"
                    )));
                }
                DexExecutionResult::Void
                | DexExecutionResult::Value(_)
                | DexExecutionResult::Throw(_) => false,
            };
            if !keep {
                let entry = (queue_key.clone(), handler);
                let mut runtime = self.runtime.borrow_mut();
                if let Some(index) = runtime
                    .idle_handlers
                    .iter()
                    .position(|known| *known == entry)
                {
                    runtime.idle_handlers.remove(index);
                }
            }
        }

        Ok(true)
    }
}
