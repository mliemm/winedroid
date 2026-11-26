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

fn handle_message_dex(class: &str, superclass: &str, boolean: bool, value: u8) -> Vec<u8> {
    const DEX_HEADER_SIZE: usize = 0x70;
    const DEX_ENDIAN_CONSTANT: u32 = 0x1234_5678;
    const STRING_ID_SIZE: usize = 4;
    const TYPE_ID_SIZE: usize = 4;
    const PROTO_ID_SIZE: usize = 12;
    const FIELD_ID_SIZE: usize = 8;
    const METHOD_ID_SIZE: usize = 8;
    const CLASS_DEF_SIZE: usize = 32;

    let return_type = if boolean { "Z" } else { "V" };
    let shorty = if boolean { "ZL" } else { "VL" };
    let strings = [
        class,
        superclass,
        "I",
        return_type,
        "Landroid/os/Message;",
        shorty,
        "handleMessage",
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

    let insns: Vec<u16> = if boolean {
        vec![0x0012 | (u16::from(value) << 12), 0x1059, 0x0000, 0x000f]
    } else {
        vec![0x0012 | (u16::from(value) << 12), 0x1059, 0x0000, 0x000e]
    };
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
