use super::super::*;
use super::helpers::minimal_class_stub_dex;

fn diagnostic_program(class: &str, returns: &[&str], strict: bool) -> DexProgram {
    let dex = dex::parse_dex_file("diagnostic.dex", minimal_class_stub_dex(class))
        .expect("fixture parses");
    let mut program = DexProgram::new(vec![dex]);
    program.strict_missing = strict;
    let methods = returns
        .iter()
        .enumerate()
        .map(|(index, return_type)| {
            Some(Rc::new(DexMethodId {
                class_descriptor: class.to_owned(),
                name: format!("missing{index}"),
                proto: dex::DexProto {
                    shorty: return_type.to_string(),
                    return_type: return_type.to_string(),
                    parameters: Vec::new(),
                },
            }))
        })
        .collect();
    program.caches.borrow_mut().method_ids = vec![methods];
    program
}

fn run_two_missing_calls(program: &DexProgram) -> Result<DexExecutionResult> {
    let code = DexCodeItem {
        registers_size: 1,
        ins_size: 0,
        outs_size: 0,
        tries_size: 0,
        debug_info_offset: 0,
        instructions: vec![
            0x0071, 0, 0, 0x000a, 0x0071, 1, 0, 0x000a, 0x0013, 42, 0x000f,
        ],
        try_blocks: Vec::new(),
    };
    let mut handler = DexRunInvocationHandler {
        program,
        current_dex_file_index: 0,
        current_class_descriptor: Some("Ldiagnostic/Caller;".to_owned()),
        depth: 0,
    };
    dex_interpreter::interpret_code_with_initial_arguments_in_context(&code, None, &mut handler, 0)
}

#[test]
fn diagnostic_policy_continues_bytecode_after_two_missing_calls() {
    let program = diagnostic_program("Ldiagnostic/Continue;", &["I", "I"], false);
    assert_eq!(
        run_two_missing_calls(&program).unwrap(),
        DexExecutionResult::Int(42)
    );
    let entries = crate::missing::recorded_entries();
    for index in 0..2 {
        let signature = format!("Ldiagnostic/Continue;.missing{index}()I");
        let entry = entries
            .iter()
            .find(|entry| entry.primitive.contains(&signature))
            .unwrap();
        assert!(entry.first_detail.contains("Ldiagnostic/Caller;"));
        assert!(entry.first_detail.contains("best-effort"));
    }
}

#[test]
fn diagnostic_policy_strict_stops_before_second_missing_call() {
    let program = diagnostic_program("Ldiagnostic/Strict;", &["I", "I"], true);
    let error = run_two_missing_calls(&program).unwrap_err().to_string();
    assert!(error.contains("Ldiagnostic/Strict;.missing0()I"));
    assert!(error.contains("Ldiagnostic/Caller;"));
    assert!(!crate::missing::recorded_entries()
        .iter()
        .any(|entry| entry.primitive.contains("Ldiagnostic/Strict;.missing1")));
}

#[test]
fn diagnostic_policy_preserves_guest_throw_after_missing_call() {
    let program = diagnostic_program("Ldiagnostic/Null;", &["I"], false);
    let mut handler = DexRunInvocationHandler {
        program: &program,
        current_dex_file_index: 0,
        current_class_descriptor: None,
        depth: 0,
    };
    let code = DexCodeItem {
        registers_size: 1,
        ins_size: 0,
        outs_size: 1,
        tries_size: 0,
        debug_info_offset: 0,
        instructions: vec![0x0071, 0, 0, 0x0012, 0x0027],
        try_blocks: Vec::new(),
    };
    let result = dex_interpreter::interpret_code_with_initial_arguments_in_context(
        &code,
        None,
        &mut handler,
        0,
    )
    .unwrap();
    assert!(matches!(result, DexExecutionResult::Throw(_)));
}

#[test]
fn diagnostic_policy_does_not_swallow_unrelated_runtime_errors() {
    let program = diagnostic_program("Ldiagnostic/RuntimeError;", &["V"], false);
    let handler = DexRunInvocationHandler {
        program: &program,
        current_dex_file_index: 0,
        current_class_descriptor: None,
        depth: 0,
    };
    let result = handler.finish_invocation(0, Err(WinedroidError::new("invalid register")));
    assert_eq!(result.unwrap_err().to_string(), "invalid register");
}

#[test]
fn diagnostic_policy_returns_declared_types_without_allocating_fake_objects() {
    let returns = [
        "V",
        "Z",
        "B",
        "C",
        "S",
        "I",
        "J",
        "D",
        "F",
        "Ljava/lang/Object;",
        "[I",
    ];
    let program = diagnostic_program("Ldiagnostic/ReturnTypes;", &returns, false);
    let mut handler = DexRunInvocationHandler {
        program: &program,
        current_dex_file_index: 0,
        current_class_descriptor: None,
        depth: 0,
    };
    for (index, descriptor) in returns.iter().enumerate() {
        let result = handler
            .invoke(DexInvocation {
                kind: DexInvokeKind::Static,
                range: false,
                method_index: index as u32,
                argument_registers: Default::default(),
                arguments: Default::default(),
            })
            .unwrap();
        let expected = match *descriptor {
            "V" => DexInvocationResult::Void,
            "J" | "D" => DexInvocationResult::Value(DexValue::Long(0)),
            "F" => DexInvocationResult::Value(DexValue::Float(0.0)),
            "Ljava/lang/Object;" | "[I" => DexInvocationResult::Value(DexValue::Null),
            _ => DexInvocationResult::Value(DexValue::Int(0)),
        };
        assert_eq!(result, expected, "{descriptor}");
    }
}

#[test]
fn diagnostic_policy_recognizes_missing_support_inside_execution_context() {
    let program = diagnostic_program("Ldiagnostic/Wrapped;", &["V"], false);
    let handler = DexRunInvocationHandler {
        program: &program,
        current_dex_file_index: 0,
        current_class_descriptor: None,
        depth: 0,
    };
    let error = WinedroidError::with_source(
        "while executing a callback",
        WinedroidError::new("next missing primitive: diagnostic wrapped callback"),
    );
    assert_eq!(
        handler.finish_invocation(0, Err(error)).unwrap(),
        DexInvocationResult::Void
    );
    let entries = crate::missing::recorded_entries();
    assert!(!entries
        .iter()
        .any(|entry| { entry.primitive == "method Ldiagnostic/Wrapped;.missing0()V" }));
    let cause = entries
        .iter()
        .find(|entry| entry.primitive == "diagnostic wrapped callback")
        .unwrap();
    assert_eq!(cause.count, 1);
    let caller = &cause.abandoned_calls["Ldiagnostic/Wrapped;.missing0()V"];
    assert_eq!(caller.count, 1);
    assert!(caller.first_detail.contains("while executing a callback"));
    assert!(caller.first_detail.contains("substitute: Void"));
}

#[test]
fn diagnostic_policy_handles_methods_without_code_in_both_modes() {
    for strict in [false, true] {
        let program = diagnostic_program("Ldiagnostic/Native;", &["J"], strict);
        let method = program.method_id(0, 0).unwrap();
        let result = program.missing_code_method(&method);
        if strict {
            assert!(result.is_err());
        } else {
            assert_eq!(
                result.unwrap(),
                DexExecutionResult::Value(DexValue::Long(0))
            );
        }
    }
}

#[test]
fn diagnostic_policy_groups_repeated_abandoned_calls_under_their_actual_cause() {
    let program = diagnostic_program("Ldiagnostic/Causes;", &["V", "V"], false);
    let handler = DexRunInvocationHandler {
        program: &program,
        current_dex_file_index: 0,
        current_class_descriptor: None,
        depth: 0,
    };
    for index in [0, 0, 1] {
        let error = WinedroidError::new("next missing primitive: diagnostic shared cause");
        handler.finish_invocation(index, Err(error)).unwrap();
    }
    let entries = crate::missing::recorded_entries();
    let cause = entries
        .iter()
        .find(|entry| entry.primitive == "diagnostic shared cause")
        .unwrap();
    assert_eq!(cause.count, 3);
    assert_eq!(cause.abandoned_calls.len(), 2);
    assert_eq!(
        cause.abandoned_calls["Ldiagnostic/Causes;.missing0()V"].count,
        2
    );
    assert_eq!(
        cause.abandoned_calls["Ldiagnostic/Causes;.missing1()V"].count,
        1
    );
    assert!(!entries
        .iter()
        .any(|entry| entry.primitive.contains("Ldiagnostic/Causes;")));
}

// cite: art common_throws.cc#ThrowNullPointerExceptionForMethodAccessImpl
// cite: art dex_file.cc#AppendPrettyMethod
#[test]
fn art_null_receiver_message_names_kind_and_pretty_method_signature() {
    let program = diagnostic_program("Lcom/example/Target;", &["Ljava/lang/String;"], false);
    program.caches.borrow_mut().method_ids = vec![vec![Some(Rc::new(DexMethodId {
        class_descriptor: "Lcom/example/Target;".to_owned(),
        name: "find".to_owned(),
        proto: dex::DexProto {
            shorty: "LI".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: vec!["I".to_owned(), "[Ljava/lang/String;".to_owned()],
        },
    }))]];
    let mut handler = DexRunInvocationHandler {
        program: &program,
        current_dex_file_index: 0,
        current_class_descriptor: None,
        depth: 0,
    };

    assert_eq!(
        handler.null_receiver_message(0, DexInvokeKind::Interface),
        Some(
            "Attempt to invoke interface method 'java.lang.String com.example.Target.find(int, java.lang.String[])' on a null object reference"
                .to_owned()
        )
    );
}
