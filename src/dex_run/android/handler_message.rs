use super::*;

const HANDLER: &str = "Landroid/os/Handler;";
const MESSAGE: &str = "Landroid/os/Message;";
const FLAG_IN_USE: i32 = 1;

fn message_flags_field() -> DexFieldId {
    message_int_field("flags")
}

fn message_callback_field() -> DexFieldId {
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

impl DexProgram {
    // cite: frameworks-base Handler.java#sendMessage, sendMessageDelayed
    pub(in crate::dex_run) fn handler_send_message(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexExecutionResult> {
        let [handler, message] = invocation.arguments.as_slice() else {
            return None;
        };

        self.handler_enqueue_message(handler, message, 0)
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

        self.handler_enqueue_message(handler, message, delay_millis)
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

        self.handler_enqueue_message(handler, &message, delay_millis)
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
            false,
        );

        Some(DexExecutionResult::Int(1))
    }

    // cite: frameworks-base Handler.java#removeMessages, getPostMessage
    // cite: frameworks-base MessageQueue.java#removeMessages
    pub(in crate::dex_run) fn handler_remove_messages(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
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

        let handler_key = map_identity_key(handler);
        let posts = std::mem::take(&mut self.runtime.borrow_mut().handler_posts);
        let (removed, kept): (Vec<_>, Vec<_>) = posts.into_iter().partition(|post| {
            if post.handler_key != handler_key {
                return false;
            }
            let (post_what, post_object) = self.queued_post_what_and_obj(&post.runnable);

            post_what == what
                && object_key
                    .as_ref()
                    .is_none_or(|key| post_object.as_ref() == Some(key))
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

        let message_callback = self.read_stored_instance_field(message, &message_callback_field());
        if let Some(callback @ (DexValue::Object(_) | DexValue::Reference(_))) = message_callback {
            self.dispatch_runnable(&callback, "Message.callback", depth)?;
        } else if !self.dispatch_handler_callback(&handler, message, depth)? {
            self.dispatch_handler_handle_message(&handler, message, depth)?;
        }

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
