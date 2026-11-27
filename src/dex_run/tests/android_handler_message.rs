use super::super::*;
use super::helpers::*;

const OVERRIDE_HANDLER: &str = "Lcom/example/OverrideHandler;";
const CALLBACK: &str = "Lcom/example/Callback;";

fn handler_method(name: &str, return_type: &str, parameters: &[&str]) -> DexMethodId {
    builtin_method("Landroid/os/Handler;", name, return_type, parameters)
}

fn seen_field(class_descriptor: &str) -> DexFieldId {
    DexFieldId {
        class_descriptor: class_descriptor.to_owned(),
        type_descriptor: "I".to_owned(),
        name: "seen".to_owned(),
    }
}

fn new_handler(program: &DexProgram, class_descriptor: &str) -> DexValue {
    let handler = program.allocate_object(class_descriptor.to_owned());
    let looper = program.allocate_object("Landroid/os/Looper;".to_owned());
    let constructor = handler_method("<init>", "V", &["Landroid/os/Looper;"]);
    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![handler.clone(), looper]))
        ),
        Some(DexExecutionResult::Void)
    );

    handler
}

fn obtain_message(program: &DexProgram, handler: &DexValue, what: i32) -> DexValue {
    let obtain = handler_method(
        "obtainMessage",
        "Landroid/os/Message;",
        &["I", "Ljava/lang/Object;"],
    );
    match program.invoke_builtin_method(
        &obtain,
        Some(&test_invocation(vec![
            handler.clone(),
            DexValue::Int(what),
            DexValue::Null,
        ])),
    ) {
        Some(DexExecutionResult::Value(message)) => message,
        other => panic!("obtainMessage: {other:?}"),
    }
}

// cite: frameworks-base Handler.java#obtainMessage
// cite: frameworks-base Message.java#obtain
#[test]
fn handler_obtain_message_overloads_set_target_and_only_the_given_fields() {
    let program = DexProgram::new(Vec::new());
    let handler = new_handler(&program, "Landroid/os/Handler;");
    let obj = program.allocate_object("Ljava/lang/Object;".to_owned());
    let obtain = |parameters: &[&str], extra: Vec<DexValue>| {
        let mut arguments = vec![handler.clone()];
        arguments.extend(extra);
        let method = handler_method("obtainMessage", "Landroid/os/Message;", parameters);
        match program.invoke_builtin_method(&method, Some(&test_invocation(arguments))) {
            Some(DexExecutionResult::Value(message)) => message,
            other => panic!("obtainMessage{parameters:?}: {other:?}"),
        }
    };
    let int_field = |message: &DexValue, name: &str| {
        program.read_stored_instance_field(message, &message_int_field(name))
    };

    let plain = obtain(&[], Vec::new());
    assert_eq!(int_field(&plain, "what"), Some(DexValue::Int(0)));
    assert_eq!(
        program.read_stored_instance_field(&plain, &message_handler_field("target")),
        Some(handler.clone())
    );

    let what_only = obtain(&["I"], vec![DexValue::Int(7)]);
    assert_eq!(int_field(&what_only, "what"), Some(DexValue::Int(7)));
    assert_eq!(int_field(&what_only, "arg1"), Some(DexValue::Int(0)));

    let full = obtain(
        &["I", "I", "I", "Ljava/lang/Object;"],
        vec![
            DexValue::Int(7),
            DexValue::Int(8),
            DexValue::Int(9),
            obj.clone(),
        ],
    );
    assert_eq!(int_field(&full, "arg1"), Some(DexValue::Int(8)));
    assert_eq!(int_field(&full, "arg2"), Some(DexValue::Int(9)));
    assert_eq!(
        program.read_stored_instance_field(&full, &message_object_field("obj")),
        Some(obj)
    );

    let ints = obtain(
        &["I", "I", "I"],
        vec![DexValue::Int(1), DexValue::Int(2), DexValue::Int(3)],
    );
    assert_eq!(int_field(&ints, "arg2"), Some(DexValue::Int(3)));
}

fn send_message(
    program: &DexProgram,
    handler: &DexValue,
    message: &DexValue,
) -> Option<DexExecutionResult> {
    let send = handler_method("sendMessage", "Z", &["Landroid/os/Message;"]);
    program.invoke_builtin_method(
        &send,
        Some(&test_invocation(vec![handler.clone(), message.clone()])),
    )
}

fn program_with(files: Vec<(&str, Vec<u8>)>) -> DexProgram {
    DexProgram::new(
        files
            .into_iter()
            .map(|(name, bytes)| crate::dex::parse_dex_file(name, bytes).expect("DEX should parse"))
            .collect(),
    )
}

fn message_field(program: &DexProgram, message: &DexValue, name: &str) -> Option<DexValue> {
    program.read_stored_instance_field(
        message,
        &DexFieldId {
            class_descriptor: "Landroid/os/Message;".to_owned(),
            type_descriptor: if name == "target" {
                "Landroid/os/Handler;".to_owned()
            } else {
                "I".to_owned()
            },
            name: name.to_owned(),
        },
    )
}

// cite: frameworks-base MessageQueue.java#removeMessages
#[test]
fn frameworks_handler_remove_messages_drops_matching_what_and_recycles_them() {
    let program = DexProgram::new(Vec::new());
    let handler = new_handler(&program, "Landroid/os/Handler;");
    let other_handler = new_handler(&program, "Landroid/os/Handler;");
    let kept = obtain_message(&program, &handler, 2);
    let removed = obtain_message(&program, &handler, 1);
    let other_target = obtain_message(&program, &other_handler, 1);
    for (target, message) in [
        (&handler, &kept),
        (&handler, &removed),
        (&other_handler, &other_target),
    ] {
        assert_eq!(
            send_message(&program, target, message),
            Some(DexExecutionResult::Int(1))
        );
    }
    let remove = handler_method("removeMessages", "V", &["I"]);

    assert_eq!(
        program.invoke_builtin_method(
            &remove,
            Some(&test_invocation(vec![handler.clone(), DexValue::Int(1)]))
        ),
        Some(DexExecutionResult::Void)
    );

    let queued = program.runtime.borrow().handler_posts.len();
    assert_eq!(queued, 2);
    assert_eq!(
        program.read_stored_instance_field(&removed, &message_handler_field("target")),
        Some(DexValue::Null)
    );
    assert_eq!(
        program.read_stored_instance_field(&kept, &message_int_field("what")),
        Some(DexValue::Int(2))
    );
}

// cite: frameworks-base MessageQueue.java#removeMessages
// cite: frameworks-base Handler.java#getPostMessage
#[test]
fn frameworks_handler_remove_messages_with_object_matches_obj_and_skips_bare_runnable_posts() {
    let program = DexProgram::new(Vec::new());
    let handler = new_handler(&program, "Landroid/os/Handler;");
    let tag = program.allocate_object("Ljava/lang/Object;".to_owned());
    let other_tag = program.allocate_object("Ljava/lang/Object;".to_owned());
    let obtain = handler_method(
        "obtainMessage",
        "Landroid/os/Message;",
        &["I", "Ljava/lang/Object;"],
    );
    let with_tag = |what: i32, tag: &DexValue| {
        let Some(DexExecutionResult::Value(message)) = program.invoke_builtin_method(
            &obtain,
            Some(&test_invocation(vec![
                handler.clone(),
                DexValue::Int(what),
                tag.clone(),
            ])),
        ) else {
            panic!("obtainMessage should return a Message");
        };
        send_message(&program, &handler, &message);
        message
    };
    let tagged = with_tag(1, &tag);
    let other_tagged = with_tag(1, &other_tag);
    let runnable = program.allocate_object("Ljava/lang/Runnable;".to_owned());
    let post = handler_method("post", "Z", &["Ljava/lang/Runnable;"]);
    program.invoke_builtin_method(
        &post,
        Some(&test_invocation(vec![handler.clone(), runnable])),
    );
    let remove = |what: i32, object: DexValue| {
        program.invoke_builtin_method(
            &handler_method("removeMessages", "V", &["I", "Ljava/lang/Object;"]),
            Some(&test_invocation(vec![
                handler.clone(),
                DexValue::Int(what),
                object,
            ])),
        )
    };

    assert_eq!(remove(1, tag), Some(DexExecutionResult::Void));
    assert_eq!(program.runtime.borrow().handler_posts.len(), 2);
    assert_eq!(
        program.read_stored_instance_field(&tagged, &message_handler_field("target")),
        Some(DexValue::Null)
    );
    assert_eq!(
        program.read_stored_instance_field(&other_tagged, &message_handler_field("target")),
        Some(handler.clone())
    );

    assert_eq!(remove(0, DexValue::Null), Some(DexExecutionResult::Void));
    assert_eq!(program.runtime.borrow().handler_posts.len(), 1);
}

#[test]
fn frameworks_handler_send_message_sets_target_queues_the_message_and_returns_true() {
    // cite: frameworks-base Handler.java#sendMessage, enqueueMessage
    let program = DexProgram::new(Vec::new());
    let handler = new_handler(&program, "Landroid/os/Handler;");
    let other_handler = new_handler(&program, "Landroid/os/Handler;");
    let message = obtain_message(&program, &other_handler, 5);

    assert_eq!(
        send_message(&program, &handler, &message),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        message_field(&program, &message, "target"),
        Some(handler.clone())
    );
    assert_eq!(program.runtime.borrow().handler_posts.len(), 1);
    assert_eq!(program.runtime.borrow().handler_posts[0].runnable, message);
}

#[test]
fn frameworks_handler_send_empty_message_delayed_queues_a_what_only_message_after_the_delay() {
    // cite: frameworks-base Handler.java#sendEmptyMessageDelayed, sendMessageDelayed
    let program = DexProgram::new(Vec::new());
    let handler = new_handler(&program, "Landroid/os/Handler;");
    let send = handler_method("sendEmptyMessageDelayed", "Z", &["I", "J"]);

    assert_eq!(
        program.invoke_builtin_method(
            &send,
            Some(&test_invocation(vec![
                handler.clone(),
                DexValue::Int(9),
                DexValue::Long(250),
                DexValue::Long(250),
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );

    let runtime = program.runtime.borrow();
    assert_eq!(runtime.handler_posts.len(), 1);
    let post = &runtime.handler_posts[0];
    assert_eq!(post.delay_millis, 250);
    let message = post.runnable.clone();
    drop(runtime);
    assert_eq!(
        message_field(&program, &message, "what"),
        Some(DexValue::Int(9))
    );
    assert_eq!(message_field(&program, &message, "target"), Some(handler));
}

#[test]
fn frameworks_handler_send_message_delayed_clamps_a_negative_delay_to_zero() {
    // cite: frameworks-base Handler.java#sendMessageDelayed
    let program = DexProgram::new(Vec::new());
    let handler = new_handler(&program, "Landroid/os/Handler;");
    let message = obtain_message(&program, &handler, 3);
    let send = handler_method("sendMessageDelayed", "Z", &["Landroid/os/Message;", "J"]);

    assert_eq!(
        program.invoke_builtin_method(
            &send,
            Some(&test_invocation(vec![
                handler,
                message,
                DexValue::Long(-5),
                DexValue::Long(-5),
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(program.runtime.borrow().handler_posts[0].delay_millis, 0);
}

#[test]
fn frameworks_message_queue_rejects_a_message_that_is_already_in_use() {
    // cite: frameworks-base MessageQueue.java#enqueueMessage
    let program = DexProgram::new(Vec::new());
    let handler = new_handler(&program, "Landroid/os/Handler;");
    let message = obtain_message(&program, &handler, 5);

    assert_eq!(
        send_message(&program, &handler, &message),
        Some(DexExecutionResult::Int(1))
    );
    let Some(DexExecutionResult::Throw(exception)) = send_message(&program, &handler, &message)
    else {
        panic!("second send should throw");
    };
    assert!(program.value_matches_type(&exception, "Ljava/lang/IllegalStateException;"));
    assert_eq!(program.runtime.borrow().handler_posts.len(), 1);
}

#[test]
fn frameworks_handler_send_message_null_message_throws_null_pointer_exception() {
    // cite: frameworks-base Handler.java#enqueueMessage
    let program = DexProgram::new(Vec::new());
    let handler = new_handler(&program, "Landroid/os/Handler;");

    let Some(DexExecutionResult::Throw(exception)) =
        send_message(&program, &handler, &DexValue::Null)
    else {
        panic!("null message should throw");
    };
    assert!(program.value_matches_type(&exception, "Ljava/lang/NullPointerException;"));
    assert!(program.runtime.borrow().handler_posts.is_empty());
}

#[test]
fn frameworks_handler_dispatch_message_calls_the_handle_message_override_then_recycles() {
    // cite: frameworks-base Handler.java#dispatchMessage
    // cite: frameworks-base Message.java#recycleUnchecked
    let program = program_with(vec![(
        "override.dex",
        handle_message_dex(OVERRIDE_HANDLER, "Landroid/os/Handler;", false, 1),
    )]);
    let handler = new_handler(&program, OVERRIDE_HANDLER);
    let message = obtain_message(&program, &handler, 9);
    send_message(&program, &handler, &message);
    for name in ["arg1", "arg2"] {
        program.write_stored_instance_field(&message, &message_int_field(name), DexValue::Int(7));
    }

    program.dispatch_handler_message(&message, 0).unwrap();

    assert_eq!(
        program.read_stored_instance_field(&handler, &seen_field(OVERRIDE_HANDLER)),
        Some(DexValue::Int(1))
    );
    assert_eq!(
        message_field(&program, &message, "arg1"),
        Some(DexValue::Int(0))
    );
    assert_eq!(
        message_field(&program, &message, "arg2"),
        Some(DexValue::Int(0))
    );
    assert_eq!(
        message_field(&program, &message, "target"),
        Some(DexValue::Null)
    );
    assert_eq!(
        message_field(&program, &message, "what"),
        Some(DexValue::Int(0))
    );
    assert_eq!(
        message_field(&program, &message, "flags"),
        Some(DexValue::Int(1))
    );
}

#[test]
fn frameworks_handler_dispatch_message_stops_when_the_callback_handles_it() {
    // cite: frameworks-base Handler.java#dispatchMessage, mCallback
    let program = program_with(vec![
        (
            "override.dex",
            handle_message_dex(OVERRIDE_HANDLER, "Landroid/os/Handler;", false, 1),
        ),
        (
            "callback.dex",
            handle_message_dex(CALLBACK, "Ljava/lang/Object;", true, 1),
        ),
    ]);
    let handler = new_handler_with_callback(&program, OVERRIDE_HANDLER);
    let callback = callback_of(&program, &handler);
    let message = obtain_message(&program, &handler, 9);
    send_message(&program, &handler, &message);

    program.dispatch_handler_message(&message, 0).unwrap();

    assert_eq!(
        program.read_stored_instance_field(&callback, &seen_field(CALLBACK)),
        Some(DexValue::Int(1))
    );
    assert_eq!(
        program.read_stored_instance_field(&handler, &seen_field(OVERRIDE_HANDLER)),
        None
    );
}

#[test]
fn frameworks_handler_dispatch_message_falls_through_when_the_callback_declines() {
    // cite: frameworks-base Handler.java#dispatchMessage, mCallback
    let program = program_with(vec![
        (
            "override.dex",
            handle_message_dex(OVERRIDE_HANDLER, "Landroid/os/Handler;", false, 1),
        ),
        (
            "callback.dex",
            handle_message_dex(CALLBACK, "Ljava/lang/Object;", true, 0),
        ),
    ]);
    let handler = new_handler_with_callback(&program, OVERRIDE_HANDLER);
    let message = obtain_message(&program, &handler, 9);
    send_message(&program, &handler, &message);

    program.dispatch_handler_message(&message, 0).unwrap();

    assert_eq!(
        program.read_stored_instance_field(&handler, &seen_field(OVERRIDE_HANDLER)),
        Some(DexValue::Int(1))
    );
}

#[test]
fn frameworks_handler_dispatch_message_runs_message_callback_instead_of_handle_message() {
    // cite: frameworks-base Handler.java#dispatchMessage, handleCallback
    let program = program_with(vec![(
        "override.dex",
        handle_message_dex(OVERRIDE_HANDLER, "Landroid/os/Handler;", false, 1),
    )]);
    let handler = new_handler(&program, OVERRIDE_HANDLER);
    let message = obtain_message(&program, &handler, 9);
    send_message(&program, &handler, &message);
    let finished_task = program.allocate_object("Ljava/util/concurrent/FutureTask;".to_owned());
    program.write_stored_instance_field(
        &finished_task,
        &crate::dex_run::android::os::future_task_done_field(),
        DexValue::Int(1),
    );
    program.write_stored_instance_field(
        &message,
        &DexFieldId {
            class_descriptor: "Landroid/os/Message;".to_owned(),
            type_descriptor: "Ljava/lang/Runnable;".to_owned(),
            name: "callback".to_owned(),
        },
        finished_task,
    );

    program.dispatch_handler_message(&message, 0).unwrap();

    assert_eq!(
        program.read_stored_instance_field(&handler, &seen_field(OVERRIDE_HANDLER)),
        None
    );
    assert_eq!(
        message_field(&program, &message, "target"),
        Some(DexValue::Null)
    );
}

#[test]
fn frameworks_handler_dispatch_message_without_a_target_is_an_error() {
    // cite: frameworks-base Looper.java#loopOnce
    let program = DexProgram::new(Vec::new());
    let message = program.allocate_object("Landroid/os/Message;".to_owned());

    assert!(program.dispatch_handler_message(&message, 0).is_err());
}

#[test]
fn frameworks_looper_thread_runs_posts_of_its_handlers_on_that_thread() {
    // cite: frameworks-base Looper.java#getThread, mThread
    let program = DexProgram::new(Vec::new());
    let thread = program.allocate_object("Landroid/os/HandlerThread;".to_owned());
    let handler = new_handler(&program, "Landroid/os/Handler;");
    let key = map_identity_key(&handler);
    let looper = program.runtime.borrow().handler_loopers[&key].clone();

    assert_eq!(program.handler_looper_thread(&key), None);
    program.write_stored_instance_field(
        &looper,
        &crate::dex_run::android::handler_message::looper_thread_field(),
        thread.clone(),
    );
    assert_eq!(program.handler_looper_thread(&key), Some(thread));
}

fn new_handler_with_callback(program: &DexProgram, class_descriptor: &str) -> DexValue {
    let handler = program.allocate_object(class_descriptor.to_owned());
    let looper = program.allocate_object("Landroid/os/Looper;".to_owned());
    let callback = program.allocate_object(CALLBACK.to_owned());
    let constructor = handler_method(
        "<init>",
        "V",
        &["Landroid/os/Looper;", "Landroid/os/Handler$Callback;"],
    );
    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![handler.clone(), looper, callback]))
        ),
        Some(DexExecutionResult::Void)
    );

    handler
}

fn callback_of(program: &DexProgram, handler: &DexValue) -> DexValue {
    program
        .read_stored_instance_field(
            handler,
            &DexFieldId {
                class_descriptor: "Landroid/os/Handler;".to_owned(),
                type_descriptor: "Landroid/os/Handler$Callback;".to_owned(),
                name: "mCallback".to_owned(),
            },
        )
        .expect("callback should be stored")
}

// cite: frameworks-base Looper.java#setMessageLogging, loopOnce
#[test]
fn looper_dispatch_calls_the_registered_printer_and_can_clear_it() {
    let printer_class = "Lcom/example/RecordingPrinter;";
    let dex = dex::parse_dex_file(
        "classes.dex",
        callback_method_dex(
            printer_class,
            "Ljava/lang/Object;",
            [
                "Ljava/lang/String;",
                "V",
                "Ljava/lang/String;",
                "VL",
                "println",
            ],
            &[0x125b, 0x0000, 0x000e],
        ),
    )
    .unwrap();
    let program = DexProgram::new(vec![dex]);
    let handler = new_handler(&program, "Landroid/os/Handler;");
    let looper = program.runtime.borrow().handler_loopers[&map_identity_key(&handler)].clone();
    let printer = program.allocate_object(printer_class.to_owned());
    let setter = builtin_method(
        "Landroid/os/Looper;",
        "setMessageLogging",
        "V",
        &["Landroid/util/Printer;"],
    );
    let seen = DexFieldId {
        class_descriptor: printer_class.to_owned(),
        type_descriptor: "Ljava/lang/String;".to_owned(),
        name: "seen".to_owned(),
    };
    let set_printer = |value| {
        assert_eq!(
            program.invoke_builtin_method(
                &setter,
                Some(&test_invocation(vec![looper.clone(), value]))
            ),
            Some(DexExecutionResult::Void)
        );
    };
    set_printer(printer.clone());
    program
        .dispatch_handler_message(&obtain_message(&program, &handler, 7), 0)
        .unwrap();
    let expected = format!(
        "<<<<< Finished to Handler (android.os.Handler) {{{:x}}} null",
        stable_hash_i32(&map_identity_key(&handler)) as u32,
    );
    assert_eq!(
        program.read_stored_instance_field(&printer, &seen),
        Some(DexValue::String(expected.into()))
    );
    set_printer(DexValue::Null);
    program.write_stored_instance_field(&printer, &seen, DexValue::Null);
    program
        .dispatch_handler_message(&obtain_message(&program, &handler, 9), 0)
        .unwrap();
    assert_eq!(
        program.read_stored_instance_field(&printer, &seen),
        Some(DexValue::Null)
    );
}

fn handle_message_dex(class: &str, superclass: &str, boolean: bool, value: u8) -> Vec<u8> {
    let (return_type, shorty, return_opcode) = if boolean {
        ("Z", "ZL", 0x000f)
    } else {
        ("V", "VL", 0x000e)
    };
    callback_method_dex(
        class,
        superclass,
        [
            "I",
            return_type,
            "Landroid/os/Message;",
            shorty,
            "handleMessage",
        ],
        &[
            0x0012 | (u16::from(value) << 12),
            0x1059,
            0x0000,
            return_opcode,
        ],
    )
}

fn callback_method_dex(
    class: &str,
    superclass: &str,
    [field_type, return_type, parameter, shorty, method_name]: [&str; 5],
    insns: &[u16],
) -> Vec<u8> {
    const DEX_HEADER_SIZE: usize = 0x70;
    const DEX_ENDIAN_CONSTANT: u32 = 0x1234_5678;
    const STRING_ID_SIZE: usize = 4;
    const TYPE_ID_SIZE: usize = 4;
    const PROTO_ID_SIZE: usize = 12;
    const FIELD_ID_SIZE: usize = 8;
    const METHOD_ID_SIZE: usize = 8;
    const CLASS_DEF_SIZE: usize = 32;

    let strings = [
        class,
        superclass,
        field_type,
        return_type,
        parameter,
        shorty,
        method_name,
        "seen",
    ];
    let string_ids_off = DEX_HEADER_SIZE;
    let type_ids_off = string_ids_off + STRING_ID_SIZE * strings.len();
    let proto_ids_off = type_ids_off + TYPE_ID_SIZE * 5;
    let field_ids_off = proto_ids_off + PROTO_ID_SIZE;
    let method_ids_off = field_ids_off + FIELD_ID_SIZE;
    let class_defs_off = method_ids_off + METHOD_ID_SIZE;
    let data_off = class_defs_off + CLASS_DEF_SIZE;
    let mut data = vec![0; data_off];
    let mut string_offsets = Vec::new();

    for value in strings {
        string_offsets.push(data.len() as u32);
        write_test_uleb128(&mut data, value.len() as u32);
        data.extend_from_slice(value.as_bytes());
        data.push(0);
    }
    while data.len() % 4 != 0 {
        data.push(0);
    }
    let parameters_off = data.len() as u32;
    data.extend_from_slice(&1u32.to_le_bytes());
    data.extend_from_slice(&4u16.to_le_bytes());
    data.extend_from_slice(&[0, 0]);

    let class_data_off = data.len() as u32;
    write_test_uleb128(&mut data, 0);
    write_test_uleb128(&mut data, 1);
    write_test_uleb128(&mut data, 0);
    write_test_uleb128(&mut data, 1);
    write_test_uleb128(&mut data, 0);
    write_test_uleb128(&mut data, JAVA_ACC_PUBLIC);
    write_test_uleb128(&mut data, 0);
    write_test_uleb128(&mut data, JAVA_ACC_PUBLIC);
    let code_off_index = data.len();
    data.extend_from_slice(&[0, 0]);

    while data.len() % 4 != 0 {
        data.push(0);
    }
    let code_off = data.len() as u32;
    assert!((0x80..0x4000).contains(&code_off));
    data[code_off_index] = ((code_off & 0x7f) as u8) | 0x80;
    data[code_off_index + 1] = (code_off >> 7) as u8;

    data.extend_from_slice(&3u16.to_le_bytes());
    data.extend_from_slice(&2u16.to_le_bytes());
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&(insns.len() as u32).to_le_bytes());
    for unit in insns {
        data.extend_from_slice(&unit.to_le_bytes());
    }

    let file_size = data.len() as u32;
    data[0..8].copy_from_slice(b"dex\n035\0");
    write_test_u32(&mut data, 32, file_size);
    write_test_u32(&mut data, 36, DEX_HEADER_SIZE as u32);
    write_test_u32(&mut data, 40, DEX_ENDIAN_CONSTANT);
    write_test_u32(&mut data, 56, strings.len() as u32);
    write_test_u32(&mut data, 60, string_ids_off as u32);
    write_test_u32(&mut data, 64, 5);
    write_test_u32(&mut data, 68, type_ids_off as u32);
    write_test_u32(&mut data, 72, 1);
    write_test_u32(&mut data, 76, proto_ids_off as u32);
    write_test_u32(&mut data, 80, 1);
    write_test_u32(&mut data, 84, field_ids_off as u32);
    write_test_u32(&mut data, 88, 1);
    write_test_u32(&mut data, 92, method_ids_off as u32);
    write_test_u32(&mut data, 96, 1);
    write_test_u32(&mut data, 100, class_defs_off as u32);
    write_test_u32(&mut data, 104, file_size - data_off as u32);
    write_test_u32(&mut data, 108, data_off as u32);

    for (index, offset) in string_offsets.iter().enumerate() {
        write_test_u32(&mut data, string_ids_off + index * STRING_ID_SIZE, *offset);
    }
    for (index, string_index) in [0u32, 1, 2, 3, 4].iter().enumerate() {
        write_test_u32(
            &mut data,
            type_ids_off + index * TYPE_ID_SIZE,
            *string_index,
        );
    }
    write_test_u32(&mut data, proto_ids_off, 5);
    write_test_u32(&mut data, proto_ids_off + 4, 3);
    write_test_u32(&mut data, proto_ids_off + 8, parameters_off);
    write_test_u16(&mut data, field_ids_off, 0);
    write_test_u16(&mut data, field_ids_off + 2, 2);
    write_test_u32(&mut data, field_ids_off + 4, 7);
    write_test_u16(&mut data, method_ids_off, 0);
    write_test_u16(&mut data, method_ids_off + 2, 0);
    write_test_u32(&mut data, method_ids_off + 4, 6);
    write_test_u32(&mut data, class_defs_off, 0);
    write_test_u32(&mut data, class_defs_off + 4, JAVA_ACC_PUBLIC);
    write_test_u32(&mut data, class_defs_off + 8, 1);
    write_test_u32(&mut data, class_defs_off + 16, u32::MAX);
    write_test_u32(&mut data, class_defs_off + 24, class_data_off);

    data
}

// cite: frameworks-base Message.java#obtain
#[test]
fn message_obtain_with_handler_and_callback_sets_target_and_callback() {
    let program = DexProgram::new(Vec::new());
    let handler = new_handler(&program, "Landroid/os/Handler;");
    let callback = program.allocate_object("Lcom/example/Runnable;".to_owned());
    let obtain = builtin_method(
        "Landroid/os/Message;",
        "obtain",
        "Landroid/os/Message;",
        &["Landroid/os/Handler;", "Ljava/lang/Runnable;"],
    );
    let message = match program.invoke_builtin_method(
        &obtain,
        Some(&test_invocation(vec![handler.clone(), callback.clone()])),
    ) {
        Some(DexExecutionResult::Value(message)) => message,
        other => panic!("Message.obtain: {other:?}"),
    };

    assert_eq!(
        program.read_stored_instance_field(&message, &message_handler_field("target")),
        Some(handler)
    );
    assert_eq!(
        program.read_stored_instance_field(
            &message,
            &DexFieldId {
                class_descriptor: "Landroid/os/Message;".to_owned(),
                type_descriptor: "Ljava/lang/Runnable;".to_owned(),
                name: "callback".to_owned(),
            }
        ),
        Some(callback)
    );
}

// cite: frameworks-base Handler.java#sendMessageAtTime
#[test]
fn handler_send_message_at_time_posts_at_the_absolute_uptime() {
    let program = DexProgram::new(Vec::new());
    let handler = new_handler(&program, "Landroid/os/Handler;");
    let message = obtain_message(&program, &handler, 9);
    let send = handler_method("sendMessageAtTime", "Z", &["Landroid/os/Message;", "J"]);
    program.runtime.borrow_mut().main_thread_uptime_millis = 1000;

    assert_eq!(
        program.invoke_builtin_method(
            &send,
            Some(&test_invocation(vec![
                handler.clone(),
                message.clone(),
                DexValue::Long(1750),
                DexValue::Long(1750),
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert!(program
        .runtime
        .borrow()
        .handler_posts
        .iter()
        .any(|post| post.runnable == message && post.delay_millis == 750));
}

// cite: frameworks-base Looper.java#loop
#[test]
fn frameworks_looper_loop_parks_background_threads_and_returns_on_main() {
    let program = DexProgram::new(Vec::new());
    let loop_method = builtin_method("Landroid/os/Looper;", "loop", "V", &[]);

    assert_eq!(
        program.invoke_builtin_method(&loop_method, Some(&test_invocation(Vec::new()))),
        Some(DexExecutionResult::Void)
    );
    let worker = program.allocate_object("Landroid/os/HandlerThread;".to_owned());
    let result = program.with_current_guest_thread(worker, || {
        program.invoke_builtin_method(&loop_method, Some(&test_invocation(Vec::new())))
    });
    assert!(matches!(result, Some(DexExecutionResult::Parked(_))));
}

// cite: frameworks-base Handler.java#sendMessageAtFrontOfQueue
#[test]
fn frameworks_handler_send_message_at_front_of_queue_sets_post_flag() {
    let program = DexProgram::new(Vec::new());
    let handler = new_handler(&program, "Landroid/os/Handler;");
    let message = obtain_message(&program, &handler, 4);
    let send = handler_method("sendMessageAtFrontOfQueue", "Z", &["Landroid/os/Message;"]);

    assert_eq!(
        program.invoke_builtin_method(
            &send,
            Some(&test_invocation(vec![handler, message.clone()]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert!(program
        .runtime
        .borrow()
        .handler_posts
        .iter()
        .any(|post| post.runnable == message && post.at_front && post.delay_millis == 0));
}

// cite: frameworks-base Message.java#Message
#[test]
fn message_constructor_accepts_object_receiver_only() {
    let program = DexProgram::new(Vec::new());
    let init = builtin_method("Landroid/os/Message;", "<init>", "V", &[]);
    let message = program.allocate_object("Landroid/os/Message;".to_owned());

    assert_eq!(
        program.invoke_builtin_method(&init, Some(&test_invocation(vec![message]))),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&init, Some(&test_invocation(vec![DexValue::Null]))),
        None
    );
}

// cite: frameworks-base Looper.java#getQueue
#[test]
fn looper_get_queue_returns_the_same_queue_each_call() {
    let program = DexProgram::new(Vec::new());
    let get_queue = builtin_method(
        "Landroid/os/Looper;",
        "getQueue",
        "Landroid/os/MessageQueue;",
        &[],
    );
    let main = DexValue::Reference(std::rc::Rc::from("Landroid/os/Looper;:main"));
    let call =
        || program.invoke_builtin_method(&get_queue, Some(&test_invocation(vec![main.clone()])));

    let Some(DexExecutionResult::Value(first)) = call() else {
        panic!("getQueue should return a queue");
    };
    assert!(program.value_matches_type(&first, "Landroid/os/MessageQueue;"));
    assert_eq!(call(), Some(DexExecutionResult::Value(first)));
}

// cite: frameworks-base MessageQueue.java#addIdleHandler, removeIdleHandler
#[test]
fn message_queue_registers_and_removes_idle_handlers() {
    let program = DexProgram::new(Vec::new());
    let add = builtin_method(
        "Landroid/os/MessageQueue;",
        "addIdleHandler",
        "V",
        &["Landroid/os/MessageQueue$IdleHandler;"],
    );
    let remove = builtin_method(
        "Landroid/os/MessageQueue;",
        "removeIdleHandler",
        "V",
        &["Landroid/os/MessageQueue$IdleHandler;"],
    );
    let queue = program.allocate_object("Landroid/os/MessageQueue;".to_owned());
    let handler = program.allocate_object("Lcom/example/Idle;".to_owned());
    let call = |method: &DexMethodId, value: DexValue| {
        program.invoke_builtin_method(method, Some(&test_invocation(vec![queue.clone(), value])))
    };

    assert_eq!(call(&add, handler.clone()), Some(DexExecutionResult::Void));
    assert_eq!(program.runtime.borrow().idle_handlers.len(), 1);
    assert!(matches!(
        call(&add, DexValue::Null),
        Some(DexExecutionResult::Throw(_))
    ));
    assert_eq!(call(&remove, handler), Some(DexExecutionResult::Void));
    assert!(program.runtime.borrow().idle_handlers.is_empty());
}

// cite: frameworks-base HandlerThread.java#run, mPriority
// cite: frameworks-base Process.java#getThreadPriority
#[test]
fn handler_thread_run_applies_its_priority() {
    let program = DexProgram::new(Vec::new());
    let thread = program.allocate_object("Landroid/os/HandlerThread;".to_owned());
    let priority_field = DexFieldId {
        class_descriptor: "Landroid/os/HandlerThread;".to_owned(),
        type_descriptor: "I".to_owned(),
        name: "mPriority".to_owned(),
    };
    program.write_stored_instance_field(&thread, &priority_field, DexValue::Int(10));
    let get = builtin_method("Landroid/os/Process;", "getThreadPriority", "I", &["I"]);

    let tid = program.with_current_guest_thread(thread.clone(), || {
        let _ = program.run_handler_thread(&thread, 0);
        program.current_guest_thread_tid()
    });

    assert_eq!(
        program.invoke_builtin_method(&get, Some(&test_invocation(vec![DexValue::Int(tid)]))),
        Some(DexExecutionResult::Int(10))
    );
}

// cite: frameworks-base MessageQueue.java#removeIdleHandler
#[test]
fn message_queue_remove_idle_handler_ignores_null() {
    let program = DexProgram::new(Vec::new());
    let remove = builtin_method(
        "Landroid/os/MessageQueue;",
        "removeIdleHandler",
        "V",
        &["Landroid/os/MessageQueue$IdleHandler;"],
    );
    let queue = program.allocate_object("Landroid/os/MessageQueue;".to_owned());

    assert_eq!(
        program.invoke_builtin_method(&remove, Some(&test_invocation(vec![queue, DexValue::Null]))),
        Some(DexExecutionResult::Void)
    );
}

fn idle_contract_program() -> DexProgram {
    let dex_file = crate::dex::parse_dex_file(
        "idle_contract.dex",
        include_bytes!("fixtures/idle_contract.dex").to_vec(),
    )
    .expect("idle contract DEX should parse");
    DexProgram::new(vec![dex_file])
}

fn idle_contract_field(name: &str, type_descriptor: &str) -> DexFieldId {
    DexFieldId {
        class_descriptor: "Lwinedroid/tests/IdleContract;".to_owned(),
        type_descriptor: type_descriptor.to_owned(),
        name: name.to_owned(),
    }
}

// cite: frameworks-base MessageQueue.java#next, addIdleHandler, IdleHandler
#[test]
fn main_loop_runs_idle_handlers_and_drops_those_returning_false() {
    let program = idle_contract_program();
    let get_queue = builtin_method(
        "Landroid/os/Looper;",
        "getQueue",
        "Landroid/os/MessageQueue;",
        &[],
    );
    let add = builtin_method(
        "Landroid/os/MessageQueue;",
        "addIdleHandler",
        "V",
        &["Landroid/os/MessageQueue$IdleHandler;"],
    );
    let main = DexValue::Reference(std::rc::Rc::from("Landroid/os/Looper;:main"));
    let Some(DexExecutionResult::Value(queue)) =
        program.invoke_builtin_method(&get_queue, Some(&test_invocation(vec![main])))
    else {
        panic!("main looper should have a queue");
    };
    let once = program.allocate_object("Lwinedroid/tests/IdleContract;".to_owned());
    let repeating = program.allocate_object("Lwinedroid/tests/IdleContract;".to_owned());
    program.write_stored_instance_field(
        &repeating,
        &idle_contract_field("keep", "Z"),
        DexValue::Int(1),
    );
    for handler in [&once, &repeating] {
        program.invoke_builtin_method(
            &add,
            Some(&test_invocation(vec![queue.clone(), handler.clone()])),
        );
    }
    let calls = |handler: &DexValue| {
        program.read_stored_instance_field(handler, &idle_contract_field("calls", "I"))
    };

    program
        .dispatch_pending_main_loop(320, 480, 0)
        .expect("main loop should run");
    assert_eq!(calls(&once), Some(DexValue::Int(1)));
    assert_eq!(calls(&repeating), Some(DexValue::Int(1)));
    assert_eq!(program.runtime.borrow().idle_handlers.len(), 1);

    program
        .dispatch_pending_main_loop(320, 480, 0)
        .expect("main loop should run");
    assert_eq!(calls(&once), Some(DexValue::Int(1)));
    assert_eq!(calls(&repeating), Some(DexValue::Int(2)));
}

// cite: frameworks-base Handler.java#hasMessages, removeMessages
#[test]
fn frameworks_handler_has_messages_tracks_queued_and_removed_messages() {
    let program = DexProgram::new(Vec::new());
    let handler = new_handler(&program, "Landroid/os/Handler;");
    let message = obtain_message(&program, &handler, 4);
    let has = handler_method("hasMessages", "Z", &["I"]);
    let remove = handler_method("removeMessages", "V", &["I"]);
    let has_what = |what: i32| {
        program.invoke_builtin_method(
            &has,
            Some(&test_invocation(vec![handler.clone(), DexValue::Int(what)])),
        )
    };

    assert_eq!(has_what(4), Some(DexExecutionResult::Int(0)));
    send_message(&program, &handler, &message);
    assert_eq!(has_what(4), Some(DexExecutionResult::Int(1)));
    assert_eq!(has_what(5), Some(DexExecutionResult::Int(0)));
    program.invoke_builtin_method(
        &remove,
        Some(&test_invocation(vec![handler.clone(), DexValue::Int(4)])),
    );
    assert_eq!(has_what(4), Some(DexExecutionResult::Int(0)));
}

// cite: frameworks-base Looper.java#myLooper, sThreadLocal
#[test]
fn looper_my_looper_is_per_thread_and_shared_with_s_thread_local() {
    let program = DexProgram::new(Vec::new());
    let my_looper = builtin_method(
        "Landroid/os/Looper;",
        "myLooper",
        "Landroid/os/Looper;",
        &[],
    );
    let thread_local_get =
        builtin_method("Ljava/lang/ThreadLocal;", "get", "Ljava/lang/Object;", &[]);
    let thread_local_set = builtin_method(
        "Ljava/lang/ThreadLocal;",
        "set",
        "V",
        &["Ljava/lang/Object;"],
    );
    let field = DexFieldId {
        class_descriptor: "Landroid/os/Looper;".to_owned(),
        type_descriptor: "Ljava/lang/ThreadLocal;".to_owned(),
        name: "sThreadLocal".to_owned(),
    };
    let thread_local = crate::dex_run::bootstrap_fields::bootstrap_static_field(&field)
        .expect("sThreadLocal should be a ThreadLocal");
    let main = DexValue::Reference(std::rc::Rc::from("Landroid/os/Looper;:main"));
    let worker = program.allocate_object("Ljava/lang/Thread;".to_owned());
    let current = || program.invoke_builtin_method(&my_looper, Some(&test_invocation(Vec::new())));

    assert_eq!(current(), Some(DexExecutionResult::Value(main.clone())));
    program.with_current_guest_thread(worker.clone(), || {
        assert_eq!(current(), Some(DexExecutionResult::Value(DexValue::Null)));
        let looper = program.allocate_object("Landroid/os/Looper;".to_owned());
        program.invoke_builtin_method(
            &thread_local_set,
            Some(&test_invocation(vec![thread_local.clone(), looper.clone()])),
        );
        assert_eq!(current(), Some(DexExecutionResult::Value(looper.clone())));
        assert_eq!(
            program.invoke_builtin_method(
                &thread_local_get,
                Some(&test_invocation(vec![thread_local.clone()]))
            ),
            Some(DexExecutionResult::Value(looper))
        );
    });
    assert_eq!(current(), Some(DexExecutionResult::Value(main)));
}

// cite: frameworks-base Looper.java#myQueue, myLooper, getQueue
#[test]
fn looper_my_queue_is_the_current_looper_queue_or_throws_without_one() {
    let program = DexProgram::new(Vec::new());
    let get_queue = builtin_method(
        "Landroid/os/Looper;",
        "getQueue",
        "Landroid/os/MessageQueue;",
        &[],
    );
    let my_queue = builtin_method(
        "Landroid/os/Looper;",
        "myQueue",
        "Landroid/os/MessageQueue;",
        &[],
    );
    let main = DexValue::Reference(std::rc::Rc::from("Landroid/os/Looper;:main"));
    let main_queue = program.invoke_builtin_method(&get_queue, Some(&test_invocation(vec![main])));
    assert!(matches!(main_queue, Some(DexExecutionResult::Value(_))));
    assert_eq!(program.invoke_builtin_method(&my_queue, None), main_queue);

    let worker = program.allocate_object("Ljava/lang/Thread;".to_owned());
    program.runtime.borrow_mut().current_guest_thread = worker;
    let Some(DexExecutionResult::Throw(exception)) = program.invoke_builtin_method(&my_queue, None)
    else {
        panic!("myQueue without a looper should throw");
    };
    assert!(program.value_matches_type(&exception, "Ljava/lang/NullPointerException;"));
}
