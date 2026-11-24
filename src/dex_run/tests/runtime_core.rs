use super::super::*;
use super::helpers::*;
use crate::dex_interpreter::DexInvokeKind;

#[test]
fn normalizes_dot_class_name_to_descriptor() {
    assert_eq!(
        normalize_class_name("com.example.MainActivity"),
        "Lcom/example/MainActivity;"
    );
}

#[test]
fn leaves_descriptor_class_name_unchanged() {
    assert_eq!(
        normalize_class_name("Lcom/example/MainActivity;"),
        "Lcom/example/MainActivity;"
    );
}

#[test]
fn dispatches_interface_invokes_to_object_receiver_class() {
    let method = DexMethodId {
        class_descriptor: "Lk8/a;".to_owned(),
        name: "invoke".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let invocation = DexInvocation {
        kind: DexInvokeKind::Interface,
        range: false,
        method_index: 0,
        argument_registers: smallvec::smallvec![0],
        arguments: smallvec::smallvec![DexValue::Object(DexObjectRef {
            id: 7,
            class_descriptor: std::rc::Rc::from("Landroidx/compose/foundation/style/c;"),
        })],
    };

    let receiver = receiver_dispatch_descriptor(&invocation).expect("method should dispatch");

    assert_eq!(receiver, "Landroidx/compose/foundation/style/c;");
    assert_eq!(method.name, "invoke");
    assert_eq!(method.proto.descriptor(), "()Ljava/lang/Object;");
}

#[test]
fn object_hash_code_is_marked_for_virtual_receiver_dispatch() {
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Object;".to_owned(),
        name: "hashCode".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };

    assert!(java_lang_object_virtual_method(&method));
}

#[derive(Clone)]
struct NullAssertTestResolver {
    allocated_type: &'static str,
    constructor_owner: &'static str,
    constructor_name: &'static str,
    constructor_parameters: &'static [&'static str],
    allocated_is_throwable: bool,
    constructor_owner_matches: bool,
}

impl Default for NullAssertTestResolver {
    fn default() -> Self {
        Self {
            allocated_type: "Ljava/lang/NullPointerException;",
            constructor_owner: "Ljava/lang/NullPointerException;",
            constructor_name: "<init>",
            constructor_parameters: &[],
            allocated_is_throwable: true,
            constructor_owner_matches: true,
        }
    }
}

impl NullAssertResolver for NullAssertTestResolver {
    fn resolve_null_assert_type(&self, _dex_file_index: usize, type_index: u32) -> Option<String> {
        (type_index == 0x0042).then(|| self.allocated_type.to_owned())
    }

    fn resolve_null_assert_method(
        &self,
        _dex_file_index: usize,
        method_index: u32,
    ) -> Option<std::rc::Rc<DexMethodId>> {
        (method_index == 0x0077).then(|| {
            std::rc::Rc::new(DexMethodId {
                class_descriptor: self.constructor_owner.to_owned(),
                name: self.constructor_name.to_owned(),
                proto: crate::dex::DexProto {
                    shorty: "V".to_owned(),
                    return_type: "V".to_owned(),
                    parameters: self
                        .constructor_parameters
                        .iter()
                        .map(|descriptor| (*descriptor).to_owned())
                        .collect(),
                },
            })
        })
    }

    fn null_assert_descriptor_matches(&self, candidate: &str, target: &str) -> bool {
        candidate == target
            || (candidate == self.allocated_type
                && target == "Ljava/lang/Throwable;"
                && self.allocated_is_throwable)
            || (candidate == self.allocated_type
                && target == self.constructor_owner
                && self.constructor_owner_matches)
    }
}

fn null_assert_test_method(parameters: &[&str]) -> ResolvedMethod {
    ResolvedMethod {
        dex_file_index: 0,
        dex_file_name: "test.dex".to_owned(),
        class_descriptor: "Lexample/Checks;".to_owned(),
        encoded_method: DexEncodedMethod {
            method_index: 0,
            access_flags: DEX_ACC_STATIC,
            code_offset: 1,
            kind: crate::dex::DexMethodKind::Direct,
        },
        method: DexMethodId {
            class_descriptor: "Lexample/Checks;".to_owned(),
            name: "requireReference".to_owned(),
            proto: crate::dex::DexProto {
                shorty: "V".to_owned(),
                return_type: "V".to_owned(),
                parameters: parameters
                    .iter()
                    .map(|descriptor| (*descriptor).to_owned())
                    .collect(),
            },
        },
    }
}

fn null_assert_test_code(
    registers_size: u16,
    ins_size: u16,
    guarded_register: u16,
    exception_register: u16,
) -> DexCodeItem {
    DexCodeItem {
        registers_size,
        ins_size,
        outs_size: 1,
        tries_size: 0,
        debug_info_offset: 0,
        instructions: vec![
            0x0038 | (guarded_register << 8),
            0x0003,
            0x000e,
            0x0022 | (exception_register << 8),
            0x0042,
            0x1070,
            0x0077,
            exception_register & 0x000f,
            0x0027 | (exception_register << 8),
        ],
        try_blocks: Vec::new(),
    }
}

#[derive(Default)]
struct NullAssertExecutionHandler {
    allocated: Option<DexNewInstance>,
    invocation: Option<DexInvocation>,
}

impl DexInvocationHandler for NullAssertExecutionHandler {
    fn invoke(&mut self, invocation: DexInvocation) -> Result<DexInvocationResult> {
        self.invocation = Some(invocation);
        Ok(DexInvocationResult::Void)
    }

    fn allocate_object(&mut self, instance: DexNewInstance) -> Result<DexNewInstanceResult> {
        self.allocated = Some(instance);
        Ok(DexNewInstanceResult::Allocated(DexValue::Object(
            DexObjectRef {
                id: 0x42,
                class_descriptor: std::rc::Rc::from("Ljava/lang/NullPointerException;"),
            },
        )))
    }
}

fn execute_null_assert_test_code(
    argument: DexValue,
) -> (DexExecutionResult, NullAssertExecutionHandler) {
    let code = null_assert_test_code(1, 1, 0, 0);
    let mut handler = NullAssertExecutionHandler::default();
    let result = crate::dex_interpreter::interpret_code_with_initial_arguments_in_context(
        &code,
        Some(&[argument]),
        &mut handler,
        0,
    )
    .expect("null-assert test code should execute");

    (result, handler)
}

struct NullAssertFrameShortcutHandler {
    resolved: ResolvedMethod,
    observation: Option<(DexInvokeKind, usize, bool)>,
}

impl DexInvocationHandler for NullAssertFrameShortcutHandler {
    fn invoke(&mut self, _invocation: DexInvocation) -> Result<DexInvocationResult> {
        Err(WinedroidError::new(
            "null-assert frame test unexpectedly materialized its invocation",
        ))
    }

    fn invoke_from_frame(
        &mut self,
        invocation: DexInvocationFrame<'_>,
    ) -> Result<DexInvocationResult> {
        let shortcut_applies = null_assert_shortcut_applies(&invocation, &self.resolved, 0)?;
        self.observation = Some((
            invocation.kind(),
            invocation.argument_count(),
            shortcut_applies,
        ));
        Ok(DexInvocationResult::Void)
    }
}

fn null_assert_materialized_invocation(
    kind: DexInvokeKind,
    arguments: Vec<DexValue>,
) -> DexInvocation {
    let argument_registers = (0..arguments.len())
        .map(|index| u16::try_from(index).expect("test argument register should fit"))
        .collect();
    DexInvocation {
        kind,
        range: false,
        method_index: 0x0077,
        argument_registers,
        arguments: arguments.into_iter().collect(),
    }
}

fn execute_null_assert_frame_invocation(
    opcode: u16,
    argument_count: u16,
    arguments: &[DexValue],
) -> (DexExecutionResult, Option<(DexInvokeKind, usize, bool)>) {
    let code = DexCodeItem {
        registers_size: argument_count,
        ins_size: argument_count,
        outs_size: argument_count,
        tries_size: 0,
        debug_info_offset: 0,
        instructions: vec![
            opcode | (argument_count << 12),
            0x0077,
            if argument_count == 2 { 0x0010 } else { 0 },
            0x000e,
        ],
        try_blocks: Vec::new(),
    };
    let mut handler = NullAssertFrameShortcutHandler {
        resolved: null_assert_test_method(&["Ljava/lang/Object;"]),
        observation: None,
    };
    let result = crate::dex_interpreter::interpret_code_with_initial_arguments_in_context(
        &code,
        Some(arguments),
        &mut handler,
        0,
    )
    .expect("null-assert frame invocation should execute");

    (result, handler.observation)
}

// cite: art interpreter_switch_impl-inl.h#IF_EQZ
#[test]
fn art_if_eqz_null_assert_runs_the_throw_body_for_null_integer_zero_and_boolean_false() {
    let expected_exception = DexValue::Object(DexObjectRef {
        id: 0x42,
        class_descriptor: std::rc::Rc::from("Ljava/lang/NullPointerException;"),
    });
    let (null_result, null_handler) = execute_null_assert_test_code(DexValue::Null);
    let (integer_zero_result, integer_zero_handler) =
        execute_null_assert_test_code(DexValue::Int(0));
    let (boolean_false_result, boolean_false_handler) =
        execute_null_assert_test_code(DexValue::Int(0));
    let non_null_reference = DexValue::Object(DexObjectRef {
        id: 7,
        class_descriptor: std::rc::Rc::from("Ljava/lang/Object;"),
    });
    let (non_null_result, non_null_handler) = execute_null_assert_test_code(non_null_reference);

    assert_eq!(
        null_result,
        DexExecutionResult::Throw(expected_exception.clone())
    );
    assert_eq!(
        integer_zero_result,
        DexExecutionResult::Throw(expected_exception.clone())
    );
    assert_eq!(
        boolean_false_result,
        DexExecutionResult::Throw(expected_exception)
    );
    assert_eq!(non_null_result, DexExecutionResult::Void);
    assert_eq!(null_handler.allocated.unwrap().type_index, 0x0042);
    assert_eq!(integer_zero_handler.allocated.unwrap().type_index, 0x0042);
    assert_eq!(boolean_false_handler.allocated.unwrap().type_index, 0x0042);
    assert!(non_null_handler.allocated.is_none());
    assert_eq!(null_handler.invocation.unwrap().kind, DexInvokeKind::Direct);
    assert_eq!(
        integer_zero_handler.invocation.unwrap().kind,
        DexInvokeKind::Direct
    );
    assert_eq!(
        boolean_false_handler.invocation.unwrap().kind,
        DexInvokeKind::Direct
    );
    assert!(non_null_handler.invocation.is_none());
}

// cite: art method_verifier.cc#SetTypesFromSignature, VerifyInvocationArgsFromIterator
// cite: art dex_instruction_list.h#INVOKE_DIRECT, INVOKE_STATIC
#[test]
fn art_null_assert_shortcut_checks_materialized_and_frame_invocations() {
    let resolved = null_assert_test_method(&["Ljava/lang/Object;"]);
    let reference = DexValue::Object(DexObjectRef {
        id: 7,
        class_descriptor: std::rc::Rc::from("Ljava/lang/Object;"),
    });
    let materialized_static =
        null_assert_materialized_invocation(DexInvokeKind::Static, vec![reference.clone()]);
    let materialized_null =
        null_assert_materialized_invocation(DexInvokeKind::Static, vec![DexValue::Null]);
    let materialized_integer_zero =
        null_assert_materialized_invocation(DexInvokeKind::Static, vec![DexValue::Int(0)]);
    let materialized_direct =
        null_assert_materialized_invocation(DexInvokeKind::Direct, vec![reference.clone()]);
    let materialized_wrong_arity = null_assert_materialized_invocation(
        DexInvokeKind::Static,
        vec![reference.clone(), reference.clone()],
    );
    let (static_result, static_observation) =
        execute_null_assert_frame_invocation(0x0071, 1, std::slice::from_ref(&reference));
    let (null_result, null_observation) =
        execute_null_assert_frame_invocation(0x0071, 1, &[DexValue::Null]);
    let (integer_zero_result, integer_zero_observation) =
        execute_null_assert_frame_invocation(0x0071, 1, &[DexValue::Int(0)]);
    let (direct_result, direct_observation) =
        execute_null_assert_frame_invocation(0x0070, 1, std::slice::from_ref(&reference));
    let (wrong_arity_result, wrong_arity_observation) =
        execute_null_assert_frame_invocation(0x0071, 2, &[reference.clone(), reference]);

    assert!(null_assert_shortcut_applies(&materialized_static, &resolved, 0).unwrap());
    assert!(!null_assert_shortcut_applies(&materialized_null, &resolved, 0).unwrap());
    assert!(!null_assert_shortcut_applies(&materialized_integer_zero, &resolved, 0).unwrap());
    assert!(!null_assert_shortcut_applies(&materialized_direct, &resolved, 0).unwrap());
    assert!(!null_assert_shortcut_applies(&materialized_wrong_arity, &resolved, 0).unwrap());
    assert_eq!(static_result, DexExecutionResult::Void);
    assert_eq!(null_result, DexExecutionResult::Void);
    assert_eq!(integer_zero_result, DexExecutionResult::Void);
    assert_eq!(direct_result, DexExecutionResult::Void);
    assert_eq!(wrong_arity_result, DexExecutionResult::Void);
    assert_eq!(static_observation, Some((DexInvokeKind::Static, 1, true)));
    assert_eq!(null_observation, Some((DexInvokeKind::Static, 1, false)));
    assert_eq!(
        integer_zero_observation,
        Some((DexInvokeKind::Static, 1, false))
    );
    assert_eq!(direct_observation, Some((DexInvokeKind::Direct, 1, false)));
    assert_eq!(
        wrong_arity_observation,
        Some((DexInvokeKind::Static, 2, false))
    );
}

// cite: art method_verifier.cc#SetTypesFromSignature
#[test]
fn art_signature_null_assert_accepts_only_reference_parameter_words() {
    let resolver = NullAssertTestResolver::default();
    let reference = null_assert_test_method(&["Ljava/lang/Object;", "Ljava/lang/String;"]);
    let reference_code = null_assert_test_code(2, 2, 0, 0);
    let array = null_assert_test_method(&["[Ljava/lang/Object;"]);
    let array_code = null_assert_test_code(1, 1, 0, 0);
    let after_wide = null_assert_test_method(&["J", "Ljava/lang/Object;"]);
    let after_wide_code = null_assert_test_code(3, 3, 2, 0);
    let invalid_array = null_assert_test_method(&["[V"]);
    let invalid_array_code = null_assert_test_code(1, 1, 0, 0);
    let string_constructor_resolver = NullAssertTestResolver {
        constructor_parameters: &["Ljava/lang/String;"],
        ..resolver.clone()
    };
    let mut string_constructor_code = reference_code.clone();
    string_constructor_code.outs_size = 2;
    string_constructor_code.instructions[5] = 0x2070;
    string_constructor_code.instructions[7] = 0x0010;

    assert_eq!(
        detect_null_assert_arg(&resolver, &reference, &reference_code),
        Some(0)
    );
    assert_eq!(
        detect_null_assert_arg(&resolver, &array, &array_code),
        Some(0)
    );
    assert_eq!(
        detect_null_assert_arg(&resolver, &after_wide, &after_wide_code),
        Some(2)
    );
    assert_eq!(
        detect_null_assert_arg(&resolver, &invalid_array, &invalid_array_code),
        None
    );
    assert_eq!(
        detect_null_assert_arg(
            &string_constructor_resolver,
            &reference,
            &string_constructor_code
        ),
        Some(0)
    );
    assert_eq!(
        detect_null_assert_arg(&resolver, &reference, &string_constructor_code),
        None
    );
}

// cite: art method_verifier.cc#SetTypesFromSignature, CodeFlowVerifyInstruction
// cite: art interpreter_switch_impl-inl.h#IF_EQZ
#[test]
fn art_signature_null_assert_rejects_integer_zero_and_boolean_false() {
    let resolver = NullAssertTestResolver::default();
    let code = null_assert_test_code(2, 2, 0, 0);
    let integer = null_assert_test_method(&["I", "Ljava/lang/String;"]);
    let boolean = null_assert_test_method(&["Z", "Ljava/lang/String;"]);
    let integer_zero = test_invocation(vec![DexValue::Int(0)]);
    let boolean_false = test_invocation(vec![DexValue::Int(0)]);

    assert_eq!(detect_null_assert_arg(&resolver, &integer, &code), None);
    assert_eq!(detect_null_assert_arg(&resolver, &boolean, &code), None);
    assert!(!null_assert_argument_is_non_null(&integer_zero, 0).unwrap());
    assert!(!null_assert_argument_is_non_null(&boolean_false, 0).unwrap());
}

// cite: art interpreter_switch_impl-inl.h#IF_EQZ
#[test]
fn art_null_assert_shortcut_requires_a_known_non_null_reference_value() {
    let null = test_invocation(vec![DexValue::Null]);
    let encoded_null = test_invocation(vec![DexValue::Int(0)]);
    let uninitialized = test_invocation(vec![DexValue::Uninitialized]);
    let string = test_invocation(vec![DexValue::String(std::rc::Rc::from("value"))]);
    let object = test_invocation(vec![DexValue::Object(DexObjectRef {
        id: 1,
        class_descriptor: std::rc::Rc::from("Ljava/lang/Object;"),
    })]);
    let reference = test_invocation(vec![DexValue::Reference(std::rc::Rc::from("p0"))]);

    assert!(!null_assert_argument_is_non_null(&null, 0).unwrap());
    assert!(!null_assert_argument_is_non_null(&encoded_null, 0).unwrap());
    assert!(!null_assert_argument_is_non_null(&uninitialized, 0).unwrap());
    assert!(!null_assert_argument_is_non_null(&null, 1).unwrap());
    assert!(null_assert_argument_is_non_null(&string, 0).unwrap());
    assert!(null_assert_argument_is_non_null(&object, 0).unwrap());
    assert!(null_assert_argument_is_non_null(&reference, 0).unwrap());
}

// cite: art dex_instruction_list.h#IF_EQZ, RETURN_VOID, NEW_INSTANCE, INVOKE_DIRECT, THROW
// cite: art method_verifier.cc#CheckBranchTarget, CodeFlowVerifyInstruction
#[test]
fn art_code_flow_null_assert_requires_the_complete_throw_shape() {
    let resolver = NullAssertTestResolver::default();
    let method = null_assert_test_method(&["Ljava/lang/Object;"]);
    let code = null_assert_test_code(1, 1, 0, 0);
    let mut wrong_branch = code.clone();
    wrong_branch.instructions[1] = 4;
    let mut missing_new_instance = code.clone();
    missing_new_instance.instructions[3] = 0;
    let mut wrong_constructor_receiver = code.clone();
    wrong_constructor_receiver.instructions[7] = 1;
    let mut wrong_throw_register = code.clone();
    wrong_throw_register.instructions[8] = 0x0127;
    let mut missing_throw = code.clone();
    missing_throw.instructions.pop();
    let mut trailing_code = code.clone();
    trailing_code.instructions.push(0);
    let mut caught_throw = code.clone();
    caught_throw.tries_size = 1;
    let mut non_static = method.clone();
    non_static.encoded_method.access_flags = 0;
    let mut non_void = method.clone();
    non_void.method.proto.return_type = "I".to_owned();
    let mut wrong_input_words = code.clone();
    wrong_input_words.ins_size = 0;
    let mut insufficient_out_words = code.clone();
    insufficient_out_words.outs_size = 0;
    let mut move_result_without_invoke = null_assert_test_code(2, 1, 1, 0);
    move_result_without_invoke.instructions.insert(5, 0x010c);
    let mut wide_result_clobbers_exception = null_assert_test_code(2, 1, 1, 1);
    wide_result_clobbers_exception
        .instructions
        .insert(8, 0x000b);
    let mut uninitialized_exception_invoked = null_assert_test_code(2, 1, 1, 0);
    uninitialized_exception_invoked
        .instructions
        .splice(5..5, [0x1071, 0x0088, 0x0000]);
    let mut uninitialized_exception_field_read = null_assert_test_code(2, 1, 1, 0);
    uninitialized_exception_field_read
        .instructions
        .splice(5..5, [0x0154, 0x0088]);
    let wrong_constructor = NullAssertTestResolver {
        constructor_name: "notConstructor",
        ..resolver.clone()
    };
    let non_throwable = NullAssertTestResolver {
        allocated_type: "Ljava/lang/String;",
        constructor_owner: "Ljava/lang/String;",
        allocated_is_throwable: false,
        ..resolver.clone()
    };
    let wrong_constructor_owner = NullAssertTestResolver {
        constructor_owner: "Ljava/lang/IllegalStateException;",
        constructor_owner_matches: false,
        ..resolver.clone()
    };
    let superclass_constructor_owner = NullAssertTestResolver {
        allocated_type: "Lexample/SpecificException;",
        constructor_owner: "Ljava/lang/RuntimeException;",
        constructor_owner_matches: true,
        ..resolver.clone()
    };

    assert_eq!(detect_null_assert_arg(&resolver, &method, &code), Some(0));
    assert_eq!(
        detect_null_assert_arg(&resolver, &method, &wrong_branch),
        None
    );
    assert_eq!(
        detect_null_assert_arg(&resolver, &method, &missing_new_instance),
        None
    );
    assert_eq!(
        detect_null_assert_arg(&resolver, &method, &wrong_constructor_receiver),
        None
    );
    assert_eq!(
        detect_null_assert_arg(&resolver, &method, &wrong_throw_register),
        None
    );
    assert_eq!(
        detect_null_assert_arg(&resolver, &method, &missing_throw),
        None
    );
    assert_eq!(
        detect_null_assert_arg(&resolver, &method, &trailing_code),
        None
    );
    assert_eq!(
        detect_null_assert_arg(&resolver, &method, &caught_throw),
        None
    );
    assert_eq!(detect_null_assert_arg(&resolver, &non_static, &code), None);
    assert_eq!(detect_null_assert_arg(&resolver, &non_void, &code), None);
    assert_eq!(
        detect_null_assert_arg(&resolver, &method, &wrong_input_words),
        None
    );
    assert_eq!(
        detect_null_assert_arg(&resolver, &method, &insufficient_out_words),
        None
    );
    assert_eq!(
        detect_null_assert_arg(&resolver, &method, &move_result_without_invoke),
        None
    );
    assert_eq!(
        detect_null_assert_arg(&resolver, &method, &wide_result_clobbers_exception),
        None
    );
    assert_eq!(
        detect_null_assert_arg(&resolver, &method, &uninitialized_exception_invoked),
        None
    );
    assert_eq!(
        detect_null_assert_arg(&resolver, &method, &uninitialized_exception_field_read),
        None
    );
    assert_eq!(
        detect_null_assert_arg(&wrong_constructor, &method, &code),
        None
    );
    assert_eq!(detect_null_assert_arg(&non_throwable, &method, &code), None);
    assert_eq!(
        detect_null_assert_arg(&wrong_constructor_owner, &method, &code),
        None
    );
    assert_eq!(
        detect_null_assert_arg(&superclass_constructor_owner, &method, &code),
        Some(0)
    );
}

// cite: art dex_instruction_list.h#NEW_INSTANCE, INVOKE_DIRECT, THROW
#[test]
fn art_code_flow_null_assert_keeps_straight_line_throw_message_work() {
    let resolver = NullAssertTestResolver::default();
    let method = null_assert_test_method(&["Ljava/lang/Object;", "Ljava/lang/String;"]);
    let mut code = null_assert_test_code(2, 2, 0, 0);
    code.outs_size = 2;
    code.instructions.splice(
        8..8,
        [
            0x011c, 0x0043, 0x106e, 0x0088, 0x0001, 0x010c, 0x2071, 0x0099, 0x0010,
        ],
    );

    assert_eq!(detect_null_assert_arg(&resolver, &method, &code), Some(0));
}

// cite: art dex_instruction_list.h#IF_NEZ, INVOKE_VIRTUAL
#[test]
fn detects_object_equals_intrinsic_from_strict_bytecode_shape() {
    let code = DexCodeItem {
        registers_size: 2,
        ins_size: 2,
        outs_size: 2,
        tries_size: 0,
        debug_info_offset: 0,
        instructions: vec![
            0x0039, 0x0008, 0x0139, 0x0004, 0x1012, 0x000f, 0x0012, 0x000f, 0x206e, 0x59b8, 0x0010,
            0x000a, 0x000f,
        ],
        try_blocks: Vec::new(),
    };
    let mut wrong_return = code.clone();
    wrong_return.instructions[12] = 0x0011;

    assert_eq!(detect_object_equals_shape(&code), Some(0x59b8));
    assert_eq!(detect_object_equals_shape(&wrong_return), None);
}

// cite: libcore Arrays.java#copyOf
// cite: libcore System.java#arraycopy
#[test]
fn detects_object_array_gap_insert_from_strict_bytecode_shape() {
    let code = DexCodeItem {
        registers_size: 8,
        ins_size: 3,
        outs_size: 5,
        tries_size: 0,
        debug_info_offset: 0,
        instructions: vec![
            0x5052, 0x0aa6, 0x70b0, 0x003a, 0x0038, 0x5154, 0x0aa5, 0x1221, 0x2037, 0x0024, 0x1221,
            0x03e1, 0x0102, 0x32b0, 0x0391, 0x0002, 0x033b, 0x0003, 0x0201, 0x0314, 0xfff7, 0x7fff,
            0x0491, 0x0302, 0x043d, 0x0009, 0x3037, 0x0006, 0x0214, 0xffff, 0x7fff, 0x0228, 0x3201,
            0x2071, 0x5b68, 0x0021, 0x000c, 0x011a, 0x3c8d, 0x2071, 0x2813, 0x0010, 0x505b, 0x0aa5,
            0x5054, 0x0aa5, 0x5152, 0x0aa6, 0x0290, 0x0706, 0x5171, 0x0d61, 0x6200, 0x5652, 0x0aa6,
            0x76b0, 0x5659, 0x0aa6, 0x000e, 0x0622, 0x11c5, 0x1070, 0x59c0, 0x0006, 0x0627,
        ],
        try_blocks: Vec::new(),
    };
    let mut wrong_size = code.clone();
    wrong_size.instructions[0] = 0x4052;

    assert_eq!(
        detect_object_array_gap_insert_shape(&code),
        Some((0x0aa5, 0x0aa6))
    );
    assert_eq!(detect_object_array_gap_insert_shape(&wrong_size), None);
}

// cite: libcore AbstractList.java#modCount
// cite: libcore Arrays.java#copyOf
#[test]
fn detects_object_array_append_from_strict_bytecode_shape() {
    let add_code = DexCodeItem {
        registers_size: 5,
        ins_size: 2,
        outs_size: 3,
        tries_size: 0,
        debug_info_offset: 0,
        instructions: vec![
            0x106e, 0x1115, 0x0003, 0x3052, 0x0aa6, 0x3152, 0x38c8, 0x1212, 0x21b0, 0x3159, 0x38c8,
            0x306e, 0x1117, 0x0203, 0x3154, 0x0aa5, 0x044d, 0x0001, 0x020f,
        ],
        try_blocks: Vec::new(),
    };
    let mutability_code = DexCodeItem {
        registers_size: 2,
        ins_size: 1,
        outs_size: 1,
        tries_size: 0,
        debug_info_offset: 0,
        instructions: vec![
            0x1055, 0x0aa7, 0x0039, 0x0003, 0x000e, 0x0022, 0x11d6, 0x1070, 0x5a48, 0x0000, 0x0027,
        ],
        try_blocks: Vec::new(),
    };
    let mut wrong_code = add_code.clone();
    wrong_code.instructions[18] = 0x000f;

    let shape = detect_object_array_append_shape(&add_code).expect("append shape");
    assert_eq!(shape.mutability_method_index, 0x1115);
    assert_eq!(shape.grow_method_index, 0x1117);
    assert_eq!(shape.array_field_index, 0x0aa5);
    assert_eq!(shape.size_field_index, 0x0aa6);
    assert_eq!(shape.mod_count_field_index, 0x38c8);
    assert_eq!(
        detect_object_array_mutability_check_shape(&mutability_code),
        Some(0x0aa7)
    );
    assert!(detect_object_array_append_shape(&wrong_code).is_none());
}

// cite: libcore System.java#arraycopy
#[test]
fn detects_slot_object_gap_ensure_from_recorded_dex_shape() {
    let ensure_code = DexCodeItem {
        registers_size: 12,
        ins_size: 3,
        outs_size: 5,
        tries_size: 0,
        debug_info_offset: 0,
        instructions: vec![
            0x0a3d, 0x0047, 0x9052, 0x25d2, 0x306e, 0x2ae3, 0x0b09, 0x9b52, 0x25d4, 0x9052, 0x25d5,
            0xa035, 0x002f, 0x9154, 0x25cc, 0x1221, 0x0391, 0x0002, 0x04da, 0x0202, 0x0590, 0x0a03,
            0x2071, 0x599b, 0x0054, 0x040a, 0x0513, 0x0020, 0x2071, 0x599b, 0x0054, 0x040a, 0x4523,
            0x1a95, 0x0612, 0x6701, 0x4735, 0x0008, 0x0812, 0x084d, 0x0705, 0x07d8, 0x0107, 0xf928,
            0x34b1, 0xb0b0, 0x0390, 0x040b, 0x5b71, 0x5a1b, 0x6561, 0x02b1, 0x5271, 0x5a1b, 0x3501,
            0x955b, 0x25cc, 0x4001, 0x9152, 0x25d3, 0xb134, 0x0005, 0xa1b0, 0x9159, 0x25d3, 0xabb0,
            0x9b59, 0x25d4, 0xa0b1, 0x9059, 0x25d5, 0x000e,
        ],
        try_blocks: Vec::new(),
    };
    let move_code = DexCodeItem {
        registers_size: 12,
        ins_size: 3,
        outs_size: 5,
        tries_size: 0,
        debug_info_offset: 0,
        instructions: vec![
            0x9052, 0x25d5, 0x9152, 0x25d4, 0x9252, 0x25d6, 0xa132, 0x0015, 0x9354, 0x25cc, 0x1a35,
            0x0009, 0x0490, 0x000a, 0xa1b1, 0x5171, 0x5a1b, 0x43a3, 0x0928, 0x0490, 0x0001, 0x0590,
            0x000a, 0x45b1, 0x5571, 0x5a1b, 0x1343, 0x1112, 0x1bb0, 0x106e, 0x2b03, 0x0009, 0x030a,
            0x2071, 0x599f, 0x003b, 0x0b0a, 0xb232, 0x0069, 0x9354, 0x25cc, 0x3321, 0x03b1, 0x0012,
            0x2b35, 0x0032, 0x206e, 0x2b06, 0x00b9, 0x040a, 0x206e, 0x2b06, 0x0029, 0x020a, 0x9552,
            0x25d0, 0x2435, 0x0054, 0x9654, 0x25cb, 0x07da, 0x0504, 0x07d8, 0x0407, 0x0644, 0x0706,
            0x063a, 0x0004, 0x1801, 0x0228, 0x0801, 0x0839, 0x0007, 0x081a, 0x32f8, 0x1071, 0x2c6c,
            0x0008, 0x9854, 0x25cb, 0x0691, 0x0603, 0x16b0, 0x667b, 0x064b, 0x0708, 0x04d8, 0x0104,
            0x5433, 0xffe0, 0x9652, 0x25d1, 0x64b0, 0xdb28, 0x206e, 0x2b06, 0x0029, 0x020a, 0x206e,
            0x2b06, 0x00b9, 0x040a, 0x4235, 0x0026, 0x9554, 0x25cb, 0x06da, 0x0502, 0x06d8, 0x0406,
            0x0544, 0x0605, 0x053b, 0x0004, 0x1701, 0x0228, 0x0701, 0x0739, 0x0007, 0x071a, 0x32f7,
            0x1071, 0x2c6c, 0x0007, 0x9754, 0x25cb, 0x35b0, 0x15b0, 0x054b, 0x0607, 0x02d8, 0x0102,
            0x9552, 0x25d0, 0x5233, 0xffe0, 0x9552, 0x25d1, 0x52b0, 0xdb28, 0x9b59, 0x25d6, 0x9a59,
            0x25d4, 0x000e,
        ],
        try_blocks: Vec::new(),
    };
    let mut wrong_code = ensure_code.clone();
    wrong_code.instructions[71] = 0x000f;

    assert_eq!(
        detect_slot_object_gap_ensure_shape(&ensure_code),
        Some((0x25d2, 0x25d4, 0x25d5, 0x25cc, 0x25d3, 0x2ae3))
    );
    let move_shape = detect_slot_object_gap_move_shape(&move_code).expect("move shape");
    assert_eq!(move_shape.object_gap_size_field_index, 0x25d5);
    assert_eq!(move_shape.object_gap_start_field_index, 0x25d4);
    assert_eq!(move_shape.object_anchor_field_index, 0x25d6);
    assert_eq!(move_shape.object_array_field_index, 0x25cc);
    assert_eq!(move_shape.group_gap_start_field_index, 0x25d0);
    assert_eq!(move_shape.group_array_field_index, 0x25cb);
    assert_eq!(move_shape.group_gap_size_field_index, 0x25d1);
    assert_eq!(detect_slot_object_gap_ensure_shape(&wrong_code), None);
}

// cite: androidx SlotTable.kt#update, rawUpdate, skip, set, dataIndexToDataAddress
#[test]
fn detects_slot_object_update_from_recorded_dex_shape() {
    let update_code = DexCodeItem {
        registers_size: 6,
        ins_size: 2,
        outs_size: 3,
        tries_size: 0,
        debug_info_offset: 0,
        instructions: vec![
            0x4052, 0x25d7, 0x1112, 0x003d, 0x0007, 0x4052, 0x25df, 0x306e, 0x2b0c, 0x0014, 0x4054,
            0x25cc, 0x4252, 0x25d2, 0x03d8, 0x0102, 0x4359, 0x25d2, 0x206e, 0x2afc, 0x0024, 0x020a,
            0x0046, 0x0200, 0x4252, 0x25d2, 0x4352, 0x25d3, 0x3236, 0x0003, 0x0628, 0x021a, 0x36fd,
            0x1071, 0x2c6c, 0x0002, 0x4254, 0x25cc, 0x4352, 0x25d2, 0x13b1, 0x206e, 0x2afc, 0x0034,
            0x010a, 0x054d, 0x0102, 0x0011,
        ],
        try_blocks: Vec::new(),
    };
    let data_address_code = DexCodeItem {
        registers_size: 4,
        ins_size: 2,
        outs_size: 0,
        tries_size: 0,
        debug_info_offset: 0,
        instructions: vec![
            0x2052, 0x25d5, 0x2152, 0x25d4, 0x1335, 0x0004, 0x0112, 0x0228, 0x1112, 0x10b2, 0x30b0,
            0x000f,
        ],
        try_blocks: Vec::new(),
    };
    let mut wrong_code = update_code.clone();
    wrong_code.instructions[47] = 0x000f;

    let shape = detect_slot_object_update_shape(&update_code).expect("update shape");
    assert_eq!(shape.insert_count_field_index, 0x25d7);
    assert_eq!(shape.insert_anchor_field_index, 0x25df);
    assert_eq!(shape.object_array_field_index, 0x25cc);
    assert_eq!(shape.slot_index_field_index, 0x25d2);
    assert_eq!(shape.slot_limit_field_index, 0x25d3);
    assert_eq!(shape.slot_gap_method_index, 0x2b0c);
    assert_eq!(shape.data_address_method_index, 0x2afc);
    assert_eq!(
        detect_slot_object_data_address_shape(&data_address_code),
        Some((0x25d5, 0x25d4))
    );
    assert!(detect_slot_object_update_shape(&wrong_code).is_none());
}

// cite: androidx SlotTable.kt#dataIndex, dataAnchorToDataIndex, capacity
#[test]
fn detects_slot_group_data_index_from_recorded_dex_shape() {
    let data_index_code = DexCodeItem {
        registers_size: 4,
        ins_size: 3,
        outs_size: 1,
        tries_size: 0,
        debug_info_offset: 0,
        instructions: vec![
            0x106e, 0x2b02, 0x0001, 0x000a, 0x0334, 0x0009, 0x1254, 0x25cc, 0x2221, 0x1352, 0x25d5,
            0x32b1, 0x020f, 0x03da, 0x0503, 0x03d8, 0x0403, 0x0244, 0x0302, 0x1352, 0x25d5, 0x1054,
            0x25cc, 0x0021, 0x023b, 0x0007, 0x30b1, 0x20b0, 0x00d8, 0x0100, 0x000f, 0x020f,
        ],
        try_blocks: Vec::new(),
    };
    let capacity_code = DexCodeItem {
        registers_size: 2,
        ins_size: 1,
        outs_size: 0,
        tries_size: 0,
        debug_info_offset: 0,
        instructions: vec![0x1054, 0x25cb, 0x0021, 0x00db, 0x0500, 0x000f],
        try_blocks: Vec::new(),
    };
    let mut wrong_code = data_index_code.clone();
    wrong_code.instructions[31] = 0x000f;

    let shape = detect_slot_group_data_index_shape(&data_index_code).expect("data index shape");
    assert_eq!(shape.capacity_method_index, 0x2b02);
    assert_eq!(shape.object_array_field_index, 0x25cc);
    assert_eq!(shape.object_gap_size_field_index, 0x25d5);
    assert_eq!(
        detect_slot_group_capacity_shape(&capacity_code),
        Some(0x25cb)
    );
    assert!(detect_slot_group_data_index_shape(&wrong_code).is_none());
}

// cite: androidx SlotTable.kt#size, capacity, groupIndexToAddress
#[test]
fn detects_slot_group_size_and_address_from_recorded_dex_shapes() {
    let size_code = DexCodeItem {
        registers_size: 3,
        ins_size: 1,
        outs_size: 1,
        tries_size: 0,
        debug_info_offset: 0,
        instructions: vec![
            0x106e, 0x2b02, 0x0002, 0x000a, 0x2152, 0x25d1, 0x10b1, 0x000f,
        ],
        try_blocks: Vec::new(),
    };
    let address_code = DexCodeItem {
        registers_size: 4,
        ins_size: 2,
        outs_size: 0,
        tries_size: 0,
        debug_info_offset: 0,
        instructions: vec![
            0x2052, 0x25d1, 0x2152, 0x25d0, 0x1335, 0x0004, 0x0112, 0x0228, 0x1112, 0x10b2, 0x30b0,
            0x000f,
        ],
        try_blocks: Vec::new(),
    };
    let mut wrong_size_code = size_code.clone();
    wrong_size_code.instructions[7] = 0x0011;

    assert_eq!(
        detect_slot_group_size_shape(&size_code),
        Some((0x2b02, 0x25d1))
    );
    assert_eq!(
        detect_slot_group_address_shape(&address_code),
        Some((0x25d1, 0x25d0))
    );
    assert!(detect_slot_group_size_shape(&wrong_size_code).is_none());
}

#[test]
fn detects_measured_cubic_cut_from_recorded_androidx_graphics_shapes_dex_shape() {
    let code = DexCodeItem {
        registers_size: 13,
        ins_size: 2,
        outs_size: 5,
        tries_size: 0,
        debug_info_offset: 0,
        instructions: vec![
            0xb052, 0x41f1, 0xb152, 0x41f2, 0x3071, 0x43a2, 0x010c, 0x0c0a, 0xb052, 0x41f2, 0xb152,
            0x41f1, 0x10c7, 0x01a7, 0x010c, 0x01c9, 0xb054, 0x41f3, 0x0254, 0x41f4, 0xb352, 0x41f0,
            0x31c8, 0x106e, 0x59b9, 0x0002, 0xb354, 0x41ef, 0x041a, 0x3b6d, 0x2071, 0x2814, 0x0043,
            0x3454, 0x41e8, 0x0512, 0x0544, 0x0504, 0x2652, 0x06b6, 0x65c7, 0x1612, 0x0444, 0x0604,
            0x2652, 0x06b7, 0x64c7, 0x2071, 0x6929, 0x0045, 0x040a, 0x0522, 0x1558, 0x5170, 0x68f3,
            0x4235, 0x0115, 0x3f80, 0x0212, 0x1401, 0x2601, 0x07a7, 0x0604, 0x0814, 0xc5ac, 0x3727,
            0x072d, 0x0807, 0x2812, 0x073d, 0x001c, 0x8782, 0x08a8, 0x0607, 0x48c6, 0x3912, 0x9982,
            0x98c9, 0x47c8, 0x67c6, 0x97c9, 0x206e, 0x68f4, 0x0085, 0x090a, 0x206e, 0x68f4, 0x0075,
            0x0a0a, 0x092e, 0x0a09, 0x093b, 0x0004, 0x7401, 0xdf28, 0x8601, 0xdd28, 0x46c6, 0x8482,
            0x46c9, 0x022e, 0x0602, 0x023c, 0x0026, 0x012e, 0x0106, 0x013c, 0x0022, 0x206e, 0x68fb,
            0x0063, 0x010c, 0x1254, 0x0941, 0x021f, 0x155a, 0x1154, 0x0942, 0x011f, 0x155a, 0x0322,
            0x155f, 0xb452, 0x41f1, 0x5c70, 0x690a, 0x4203, 0x0222, 0x155f, 0xb452, 0x41f2, 0x5470,
            0x690a, 0xc102, 0x0c22, 0x0298, 0x3070, 0x0c41, 0x023c, 0x0c11, 0x0c22, 0x11b1, 0x001a,
            0x0ac3, 0x2070, 0x5954, 0x000c, 0x0c27,
        ],
        try_blocks: Vec::new(),
    };
    let mut wrong_code = code.clone();
    wrong_code.instructions[147] = 0x000e;

    let shape = detect_measured_cubic_cut_shape(&code).expect("cut shape");

    assert_eq!(shape.cubic_field_index, 0x41ef);
    assert_eq!(shape.size_field_index, 0x41f0);
    assert_eq!(shape.start_field_index, 0x41f1);
    assert_eq!(shape.end_field_index, 0x41f2);
    assert_eq!(shape.owner_field_index, 0x41f3);
    assert_eq!(shape.owner_center_field_index, 0x41f4);
    assert_eq!(shape.center_x_field_index, 0x06b6);
    assert_eq!(shape.center_y_field_index, 0x06b7);
    assert_eq!(shape.cubic_points_field_index, 0x41e8);
    assert_eq!(shape.path_delta_method_index, 0x68f4);
    assert!(detect_measured_cubic_cut_shape(&wrong_code).is_none());
}

// cite: androidx Cubic.kt#transformed, transformOnePoint
#[test]
fn detects_cubic_transform_from_recorded_androidx_graphics_shapes_dex_shape() {
    let transform_code = DexCodeItem {
        registers_size: 7,
        ins_size: 2,
        outs_size: 5,
        tries_size: 0,
        debug_info_offset: 0,
        instructions: vec![
            0x0022, 0x1562, 0x0113, 0x0008, 0x1123, 0x19ae, 0x2070, 0x68f7, 0x0010, 0x5254, 0x41e8,
            0x2321, 0x0412, 0x5371, 0x5a1b, 0x4142, 0x306e, 0x6914, 0x0460, 0x2112, 0x306e, 0x6914,
            0x0160, 0x4112, 0x306e, 0x6914, 0x0160, 0x6112, 0x306e, 0x6914, 0x0160, 0x0011,
        ],
        try_blocks: Vec::new(),
    };
    let transform_point_code = DexCodeItem {
        registers_size: 10,
        ins_size: 3,
        outs_size: 3,
        tries_size: 0,
        debug_info_offset: 0,
        instructions: vec![
            0x7054, 0x41e8, 0x0144, 0x0900, 0x02d8, 0x0109, 0x0344, 0x0200, 0x3072, 0x6915, 0x0318,
            0x030b, 0x0813, 0x0020, 0x05a4, 0x0803, 0x5884, 0x1071, 0x594d, 0x0008, 0x080a, 0x084b,
            0x0900, 0x0818, 0xffff, 0xffff, 0x0000, 0x0000, 0x38c0, 0x8884, 0x1071, 0x594d, 0x0008,
            0x080a, 0x084b, 0x0200, 0x000e,
        ],
        try_blocks: Vec::new(),
    };
    let mut wrong_code = transform_code.clone();
    wrong_code.instructions[31] = 0x000f;

    let shape = detect_cubic_transform_shape(&transform_code).expect("transform shape");
    assert_eq!(shape.mutable_cubic_type_index, 0x1562);
    assert_eq!(shape.float_array_type_index, 0x19ae);
    assert_eq!(shape.constructor_method_index, 0x68f7);
    assert_eq!(shape.points_field_index, 0x41e8);
    assert_eq!(shape.arraycopy_method_index, 0x5a1b);
    assert_eq!(shape.transform_point_method_index, 0x6914);
    let transform_point_shape = detect_mutable_cubic_transform_point_shape(&transform_point_code)
        .expect("transform point shape");
    assert_eq!(transform_point_shape.points_field_index, 0x41e8);
    assert_eq!(transform_point_shape.transform_method_index, 0x6915);
    assert!(detect_cubic_transform_shape(&wrong_code).is_none());
}

// cite: androidx Features.kt#Edge, transformed
#[test]
fn detects_feature_edge_transform_from_recorded_androidx_graphics_shapes_dex_shape() {
    let code = DexCodeItem {
        registers_size: 7,
        ins_size: 2,
        outs_size: 2,
        tries_size: 0,
        debug_info_offset: 0,
        instructions: vec![
            0x0071, 0x2199, 0x0000, 0x000c, 0x5154, 0x41ee, 0x1072, 0x5c0a, 0x0001, 0x020a, 0x0312,
            0x2335, 0x0012, 0x2072, 0x5bff, 0x0031, 0x040c, 0x041f, 0x155a, 0x206e, 0x68fc, 0x0064,
            0x040c, 0x206e, 0x110c, 0x0040, 0x03d8, 0x0103, 0xef28, 0x1071, 0x2195, 0x0000, 0x060c,
            0x0022, 0x155d, 0x011a, 0x3cc8, 0x2071, 0x2814, 0x0016, 0x2070, 0x6908, 0x0060, 0x0011,
        ],
        try_blocks: Vec::new(),
    };
    let mut wrong_code = code.clone();
    wrong_code.instructions[43] = 0x000e;

    let shape = detect_feature_edge_transform_shape(&code).expect("edge transform shape");

    assert_eq!(shape.list_factory_method_index, 0x2199);
    assert_eq!(shape.cubics_field_index, 0x41ee);
    assert_eq!(shape.list_size_method_index, 0x5c0a);
    assert_eq!(shape.list_get_method_index, 0x5bff);
    assert_eq!(shape.cubic_type_index, 0x155a);
    assert_eq!(shape.cubic_transform_method_index, 0x68fc);
    assert_eq!(shape.list_add_method_index, 0x110c);
    assert_eq!(shape.list_freeze_method_index, 0x2195);
    assert_eq!(shape.edge_type_index, 0x155d);
    assert_eq!(shape.cubics_string_index, 0x3cc8);
    assert_eq!(shape.constructor_method_index, 0x6908);
    assert!(detect_feature_edge_transform_shape(&wrong_code).is_none());
}

// cite: androidx Features.kt#Corner, transformed
#[test]
fn detects_feature_corner_transform_from_recorded_androidx_graphics_shapes_dex_shape() {
    let code = DexCodeItem {
        registers_size: 13,
        ins_size: 2,
        outs_size: 7,
        tries_size: 0,
        debug_info_offset: 0,
        instructions: vec![
            0x0071, 0x2199, 0x0000, 0x000c, 0xb154, 0x41ee, 0x1072, 0x5c0a, 0x0001, 0x020a, 0x0312,
            0x2335, 0x0012, 0x2072, 0x5bff, 0x0031, 0x040c, 0x041f, 0x155a, 0x206e, 0x68fc, 0x00c4,
            0x040c, 0x206e, 0x110c, 0x0040, 0x03d8, 0x0103, 0xef28, 0x1071, 0x2195, 0x0000, 0x050c,
            0xb053, 0x41eb, 0x3071, 0x509a, 0x0c10, 0x060b, 0xb053, 0x41ec, 0x3071, 0x509a, 0x0c10,
            0x080b, 0x0422, 0x155c, 0xba55, 0x41ed, 0x0776, 0x6903, 0x0004, 0x0411,
        ],
        try_blocks: Vec::new(),
    };
    let mut wrong_code = code.clone();
    wrong_code.instructions[52] = 0x0010;

    let shape = detect_feature_corner_transform_shape(&code).expect("corner transform shape");

    assert_eq!(shape.list_factory_method_index, 0x2199);
    assert_eq!(shape.cubics_field_index, 0x41ee);
    assert_eq!(shape.list_size_method_index, 0x5c0a);
    assert_eq!(shape.list_get_method_index, 0x5bff);
    assert_eq!(shape.cubic_type_index, 0x155a);
    assert_eq!(shape.cubic_transform_method_index, 0x68fc);
    assert_eq!(shape.list_add_method_index, 0x110c);
    assert_eq!(shape.list_freeze_method_index, 0x2195);
    assert_eq!(shape.vertex_field_index, 0x41eb);
    assert_eq!(shape.point_transform_method_index, 0x509a);
    assert_eq!(shape.rounded_center_field_index, 0x41ec);
    assert_eq!(shape.corner_type_index, 0x155c);
    assert_eq!(shape.convex_field_index, 0x41ed);
    assert_eq!(shape.constructor_method_index, 0x6903);
    assert!(detect_feature_corner_transform_shape(&wrong_code).is_none());
}

// cite: androidx FeatureMapping.kt#featureDistSquared, featureRepresentativePoint
// cite: androidx Point.kt#getDistanceSquared
#[test]
fn detects_feature_distance_squared_from_recorded_androidx_graphics_shapes_dex_shape() {
    let code = DexCodeItem {
        registers_size: 7,
        ins_size: 2,
        outs_size: 2,
        tries_size: 0,
        debug_info_offset: 0,
        instructions: vec![
            0x001a, 0x3dfe, 0x2071, 0x2814, 0x0005, 0x001a, 0x3dff, 0x2071, 0x2814, 0x0006, 0x5020,
            0x155c, 0x0038, 0x0016, 0x6020, 0x155c, 0x0038, 0x0012, 0x5007, 0x001f, 0x155c, 0x6107,
            0x011f, 0x155c, 0x0055, 0x41ed, 0x1155, 0x41ed, 0x1032, 0x0006, 0x0514, 0xffff, 0x7f7f,
            0x050f, 0x5554, 0x41ee, 0x1071, 0x0d85, 0x0005, 0x000c, 0x001f, 0x155a, 0x0054, 0x41e8,
            0x0112, 0x0044, 0x0100, 0x1071, 0x0d8c, 0x0005, 0x020c, 0x021f, 0x155a, 0x106e, 0x68f8,
            0x0002, 0x020a, 0x02c6, 0x0015, 0x4000, 0x02c9, 0x1071, 0x0d85, 0x0005, 0x030c, 0x031f,
            0x155a, 0x3354, 0x41e8, 0x1412, 0x0344, 0x0403, 0x1071, 0x0d8c, 0x0005, 0x050c, 0x051f,
            0x155a, 0x106e, 0x68f9, 0x0005, 0x050a, 0x35c6, 0x05c9, 0x6654, 0x41ee, 0x1071, 0x0d85,
            0x0006, 0x030c, 0x031f, 0x155a, 0x3354, 0x41e8, 0x0144, 0x0103, 0x1071, 0x0d8c, 0x0006,
            0x030c, 0x031f, 0x155a, 0x106e, 0x68f8, 0x0003, 0x030a, 0x13c6, 0x03c9, 0x1071, 0x0d85,
            0x0006, 0x010c, 0x011f, 0x155a, 0x1154, 0x41e8, 0x0144, 0x0401, 0x1071, 0x0d8c, 0x0006,
            0x060c, 0x061f, 0x155a, 0x106e, 0x68f9, 0x0006, 0x060a, 0x16c6, 0x06c9, 0x32c7, 0x65c7,
            0x22c8, 0x55c8, 0x25c6, 0x050f,
        ],
        try_blocks: Vec::new(),
    };
    let mut wrong_code = code.clone();
    wrong_code.instructions[130] = 0x22c7;

    let shape = detect_feature_distance_squared_shape(&code).expect("feature distance shape");

    assert_eq!(shape.left_string_index, 0x3dfe);
    assert_eq!(shape.right_string_index, 0x3dff);
    assert_eq!(shape.null_check_method_index, 0x2814);
    assert_eq!(shape.corner_type_index, 0x155c);
    assert_eq!(shape.convex_field_index, 0x41ed);
    assert_eq!(shape.cubics_field_index, 0x41ee);
    assert_eq!(shape.cubic_type_index, 0x155a);
    assert_eq!(shape.cubic_points_field_index, 0x41e8);
    assert_eq!(shape.list_first_method_index, 0x0d85);
    assert_eq!(shape.list_last_method_index, 0x0d8c);
    assert_eq!(shape.cubic_anchor_x_method_index, 0x68f8);
    assert_eq!(shape.cubic_anchor_y_method_index, 0x68f9);
    assert!(detect_feature_distance_squared_shape(&wrong_code).is_none());
}

// cite: androidx Features.kt#Corner, transformed
#[test]
fn detects_packed_point_transform_from_recorded_androidx_graphics_shapes_dex_shape() {
    let code = DexCodeItem {
        registers_size: 5,
        ins_size: 3,
        outs_size: 3,
        tries_size: 0,
        debug_info_offset: 0,
        instructions: vec![
            0x2071, 0x50b3, 0x0032, 0x000a, 0x2071, 0x50b4, 0x0032, 0x020a, 0x3072, 0x6915, 0x0204,
            0x020b, 0x0413, 0x0020, 0x00a4, 0x0402, 0x0484, 0x1071, 0x594d, 0x0004, 0x040a, 0x0018,
            0xffff, 0xffff, 0x0000, 0x0000, 0x02c0, 0x2284, 0x1071, 0x594d, 0x0002, 0x020a, 0x2071,
            0x6ac9, 0x0024, 0x020b, 0x0210,
        ],
        try_blocks: Vec::new(),
    };
    let mut wrong_code = code.clone();
    wrong_code.instructions[29] = 0x594e;

    let shape = detect_packed_point_transform_shape(&code).expect("packed point transform shape");

    assert_eq!(shape.x_method_index, 0x50b3);
    assert_eq!(shape.y_method_index, 0x50b4);
    assert_eq!(shape.transform_method_index, 0x6915);
    assert_eq!(shape.int_bits_to_float_method_index, 0x594d);
    assert_eq!(shape.pack_method_index, 0x6ac9);
    assert!(detect_packed_point_transform_shape(&wrong_code).is_none());
}

// cite: androidx Matrix.kt#map
#[test]
fn detects_matrix_point_transform_from_recorded_compose_matrix_dex_shape() {
    let code = DexCodeItem {
        registers_size: 4,
        ins_size: 3,
        outs_size: 3,
        tries_size: 0,
        debug_info_offset: 0,
        instructions: vec![
            0x1054, 0x21d6, 0x2071, 0x5145, 0x0032, 0x020b, 0x3071, 0x6cdf, 0x0032, 0x020b, 0x2071,
            0x6b3d, 0x0032, 0x000a, 0x2071, 0x6b3e, 0x0032, 0x020a, 0x2071, 0x6ac9, 0x0020, 0x020b,
            0x0210,
        ],
        try_blocks: Vec::new(),
    };
    let mut wrong_code = code.clone();
    wrong_code.instructions[6] = 0x2071;

    let shape = detect_matrix_point_transform_shape(&code).expect("matrix point transform shape");

    assert_eq!(shape.receiver_arg, 0);
    assert_eq!(shape.x_arg, 1);
    assert_eq!(shape.y_arg, 2);
    assert_eq!(shape.matrix_register, 0);
    assert_eq!(shape.matrix_field_index, 0x21d6);
    assert!(detect_matrix_point_transform_shape(&wrong_code).is_none());
}

// cite: androidx PolygonMeasure.kt#measurePolygon
#[test]
fn detects_measured_polygon_measure_from_recorded_androidx_graphics_shapes_dex_shape() {
    let code = DexCodeItem {
        registers_size: 14,
        ins_size: 2,
        outs_size: 5,
        tries_size: 0,
        debug_info_offset: 0,
        instructions: vec![
            0x0022, 0x124b, 0x1070, 0x5b48, 0x0000, 0x0122, 0x124b, 0x1070, 0x5b48, 0x0001, 0xdd54,
            0x4206, 0x1072, 0x5c0a, 0x000d, 0x020a, 0x0312, 0x3401, 0x2512, 0x2435, 0x003b, 0x2072,
            0x5bff, 0x004d, 0x060c, 0x061f, 0x155e, 0x6754, 0x41ee, 0x1072, 0x5c0a, 0x0007, 0x070a,
            0x3801, 0x7835, 0x0029, 0x6920, 0x155c, 0x6a54, 0x41ee, 0x0938, 0x0019, 0x1072, 0x5c0a,
            0x000a, 0x090a, 0x59b3, 0x9833, 0x0012, 0x106e, 0x5b5c, 0x0000, 0x090a, 0x1071, 0x5971,
            0x0009, 0x090c, 0x0b22, 0x0298, 0x3070, 0x0c41, 0x096b, 0x206e, 0x5b4c, 0x00b1, 0x2072,
            0x5bff, 0x008a, 0x090c, 0x206e, 0x5b4c, 0x0090, 0x08d8, 0x0108, 0xd828, 0x04d8, 0x0104,
            0xc528, 0x0d12, 0x1071, 0x5951, 0x000d, 0x020c, 0x0413, 0x0009, 0x2071, 0x0d99, 0x0040,
            0x040a, 0x0439, 0x0007, 0x1071, 0x217c, 0x0002, 0x0d0c, 0x3928, 0x0622, 0x124b, 0x04d8,
            0x0104, 0x2070, 0x5b49, 0x0046, 0x206e, 0x5b4c, 0x0026, 0x106e, 0x5b55, 0x0000, 0x040c,
            0x1072, 0x5be5, 0x0004, 0x070a, 0x0738, 0x0025, 0x1072, 0x5be6, 0x0004, 0x070c, 0x071f,
            0x155a, 0x106e, 0x59b1, 0x0002, 0x020a, 0x206e, 0x08ae, 0x007c, 0x070a, 0x082d, 0x0d07,
            0x083a, 0x000b, 0x72c6, 0x1071, 0x5951, 0x0002, 0x020c, 0x206e, 0x5b4c, 0x0026, 0xe028,
            0x0c22, 0x11b1, 0x0d1a, 0x2be0, 0x2070, 0x5954, 0x00dc, 0x0c27, 0x6d07, 0x1071, 0x0d8c,
            0x000d, 0x020c, 0x021f, 0x11c2, 0x106e, 0x59b1, 0x0002, 0x020a, 0x0422, 0x15bd, 0x1072,
            0x5c0a, 0x000d, 0x060a, 0x2070, 0x6b08, 0x0064, 0x1072, 0x5c0a, 0x000d, 0x060a, 0x3701,
            0x6735, 0x0013, 0x2072, 0x5bff, 0x007d, 0x080c, 0x081f, 0x11c2, 0x106e, 0x59b1, 0x0008,
            0x080a, 0x28c9, 0x206e, 0x6b09, 0x0084, 0x07d8, 0x0107, 0xee28, 0x0071, 0x2199, 0x0000,
            0x0d0c, 0x106e, 0x5b5c, 0x0001, 0x020a, 0x2335, 0x0032, 0x206e, 0x5b52, 0x0031, 0x060c,
            0x061f, 0x0298, 0x6654, 0x0942, 0x061f, 0x11c2, 0x106e, 0x59b2, 0x0006, 0x060a, 0x0722,
            0x1564, 0x206e, 0x6b0a, 0x0064, 0x080a, 0x06d8, 0x0106, 0x206e, 0x6b0a, 0x0064, 0x060a,
            0x86c6, 0x5882, 0x86c9, 0x206e, 0x5b52, 0x0031, 0x080c, 0x081f, 0x0298, 0x8854, 0x0941,
            0x081f, 0x155e, 0x3070, 0x6916, 0x0867, 0x206e, 0x110c, 0x007d, 0x03d8, 0x0103, 0xcf28,
            0x1071, 0x2195, 0x000d, 0x0d0c, 0x0122, 0x1560, 0x5470, 0x690d, 0x0dc1, 0x0111,
        ],
        try_blocks: Vec::new(),
    };
    let mut wrong_code = code.clone();
    wrong_code.instructions[126] = 0x106e;

    let shape = detect_measured_polygon_measure_shape(&code).expect("measure polygon shape");

    assert_eq!(shape.polygon_features_field_index, 0x4206);
    assert_eq!(shape.feature_cubics_field_index, 0x41ee);
    assert_eq!(shape.corner_type_index, 0x155c);
    assert_eq!(shape.measure_cubic_method_index, 0x08ae);
    assert_eq!(shape.list_size_method_index, 0x5c0a);
    assert_eq!(shape.list_get_method_index, 0x5bff);
    assert_eq!(shape.array_list_add_method_index, 0x5b4c);
    assert_eq!(shape.array_list_size_method_index, 0x5b5c);
    assert_eq!(shape.array_list_get_method_index, 0x5b52);
    assert_eq!(shape.float_value_of_method_index, 0x5951);
    assert_eq!(shape.number_float_value_method_index, 0x59b1);
    assert_eq!(shape.number_int_value_method_index, 0x59b2);
    assert_eq!(shape.list_factory_method_index, 0x2199);
    assert_eq!(shape.list_add_method_index, 0x110c);
    assert_eq!(shape.list_freeze_method_index, 0x2195);
    assert_eq!(shape.progress_feature_constructor_method_index, 0x6916);
    assert_eq!(shape.measured_polygon_constructor_method_index, 0x690d);
    assert!(detect_measured_polygon_measure_shape(&wrong_code).is_none());
}

// cite: androidx PolygonMeasure.kt#Measurer, measureCubic
#[test]
fn detects_angular_measure_cubic_from_recorded_androidx_graphics_shapes_dex_shape() {
    let code = DexCodeItem {
        registers_size: 6,
        ins_size: 2,
        outs_size: 2,
        tries_size: 0,
        debug_info_offset: 0,
        instructions: vec![
            0x001a, 0x3b6d, 0x2071, 0x2814, 0x0005, 0x106e, 0x68f8, 0x0005, 0x000a, 0x4152, 0x06b6,
            0x10c7, 0x106e, 0x68f9, 0x0005, 0x020a, 0x4352, 0x06b7, 0x32c7, 0x2071, 0x6929, 0x0020,
            0x000a, 0x0212, 0x5554, 0x41e8, 0x0244, 0x0205, 0x12c7, 0x1112, 0x0544, 0x0105, 0x35c7,
            0x2071, 0x6929, 0x0052, 0x050a, 0x50c7, 0x0560, 0x420c, 0x2071, 0x692c, 0x0050, 0x000a,
            0x0114, 0xb717, 0x38d1, 0x15c7, 0x052d, 0x0500, 0x053d, 0x0004, 0x0512, 0x050f, 0x000f,
        ],
        try_blocks: Vec::new(),
    };
    let mut wrong_code = code.clone();
    wrong_code.instructions[24] = 0x5654;

    let shape = detect_angular_measure_cubic_shape(&code).expect("angular measure cubic shape");

    assert_eq!(shape.anchor_x_method_index, 0x68f8);
    assert_eq!(shape.anchor_y_method_index, 0x68f9);
    assert_eq!(shape.angle_method_index, 0x6929);
    assert_eq!(shape.positive_modulo_method_index, 0x692c);
    assert_eq!(shape.cubic_points_field_index, 0x41e8);
    assert_eq!(shape.center_x_field_index, 0x06b6);
    assert_eq!(shape.center_y_field_index, 0x06b7);
    assert!(detect_angular_measure_cubic_shape(&wrong_code).is_none());
}

// cite: androidx RoundedPolygon.kt#RoundedPolygon, cubics
// cite: androidx Cubic.kt#zeroLength, split
#[test]
fn detects_rounded_polygon_constructor_from_recorded_androidx_graphics_shapes_dex_shape() {
    let instructions = "0008 0012 0108 0013 021a 3e0e 2071 2814 0021 1070 59b6 0000 015b 4206 0202 0014 0259 4207 0202 0015 0259 4208 0071 2199 0000 020c 1072 5c0a 0001 030a 3412 2512 1612 0712 0812 033d 0059 2072 5bff 0071 030c 031f 155e 3354 41ee 1072 5c0a 0003 030a 4333 004b 2072 5bff 0071 030c 031f 155e 3354 41ee 2072 5bff 0063 030c 031f 155a 0915 3f00 206e 68fb 0093 030c 3954 0941 091f 155a 3354 0942 031f 155a 5a23 1acd 2072 5bff 0071 0b0c 0b1f 155e bb54 41ee 2072 5bff 007b 0b0c 0b4d 070a 094d 060a 1071 0d95 000a 090c 5a23 1acd 034d 070a 2072 5bff 0071 030c 031f 155e 3354 41ee 2072 5bff 0053 030c 034d 060a 1071 0d95 000a 030c 0328 8307 3907 1072 5c0a 0001 010a 013a 006d 7b01 8a07 0b39 0006 0338 0004 3c07 1c28 0c54 4206 1072 5c0a 000c 0c0a cb33 000b 0939 0007 1402 0004 8107 a807 5828 9c07 0b28 0c54 4206 2072 5bff 00bc 0c0c 0c1f 155e cc54 41ee 1072 5c0a 000c 0d0a 7e01 de35 003a 2072 5bff 00ec 0f0c 0f1f 155a 106e 68fe 000f 100a 1039 0010 0a38 0005 206e 110c 00a2 1402 0004 0839 0005 f807 8a07 1c28 fa07 1a28 0a38 0017 106e 68f8 000f 100a 1402 0004 a454 41e8 1113 0006 104b 1104 1013 0007 106e 68f9 000f 0f0a 0f4b 1004 0328 1402 0004 0ed8 010e 0402 0014 c728 1402 0004 1b32 ffb0 0bd8 010b 0402 0014 9828 1402 0004 8107 0838 0021 0138 001f 8354 41e8 0844 0703 0944 0603 0a44 0503 0b44 1403 4412 0c44 0403 5412 0d44 0403 1154 41e8 0e44 0701 0f44 0601 0877 4f7c 0008 010c 206e 110c 0012 1071 2195 0002 010c 015b 4209 106e 110a 0001 020a 62b1 206e 1116 0021 020c 106e 110a 0001 010a 7301 1335 003d 0454 4209 206e 1116 0034 040c 041f 155a 4554 41e8 0544 0705 021f 155a 106e 68f8 0002 080a 85c7 1071 5989 0005 050a 0814 b717 38d1 052d 0805 053c 0017 4554 41e8 0544 0605 106e 68f9 0002 020a 25c7 1071 5989 0005 020a 022d 0802 023c 0006 03d8 0103 4207 cc28 0122 11b1 021a 2f10 2070 5954 0021 0127 000e";
    let code = DexCodeItem {
        registers_size: 22,
        ins_size: 4,
        outs_size: 8,
        tries_size: 0,
        debug_info_offset: 0,
        instructions: instructions
            .split_whitespace()
            .map(|word| u16::from_str_radix(word, 16).expect("hex instruction"))
            .collect(),
        try_blocks: Vec::new(),
    };
    let mut wrong_code = code.clone();
    wrong_code.instructions[180] = 0x206e;

    let shape =
        detect_rounded_polygon_constructor_shape(&code).expect("rounded polygon constructor shape");

    assert_eq!(shape.features_string_index, 0x3e0e);
    assert_eq!(shape.features_field_index, 0x4206);
    assert_eq!(shape.center_x_field_index, 0x4207);
    assert_eq!(shape.center_y_field_index, 0x4208);
    assert_eq!(shape.cubics_field_index, 0x4209);
    assert_eq!(shape.feature_type_index, 0x155e);
    assert_eq!(shape.feature_cubics_field_index, 0x41ee);
    assert_eq!(shape.cubic_type_index, 0x155a);
    assert_eq!(shape.cubic_split_method_index, 0x68fb);
    assert_eq!(shape.cubic_zero_length_method_index, 0x68fe);
    assert_eq!(shape.cubic_anchor_x_method_index, 0x68f8);
    assert_eq!(shape.cubic_anchor_y_method_index, 0x68f9);
    assert_eq!(shape.cubic_points_field_index, 0x41e8);
    assert_eq!(shape.cubic_factory_method_index, 0x4f7c);
    assert_eq!(shape.list_factory_method_index, 0x2199);
    assert_eq!(shape.list_size_method_index, 0x5c0a);
    assert_eq!(shape.list_get_method_index, 0x5bff);
    assert_eq!(shape.list_add_method_index, 0x110c);
    assert_eq!(shape.list_freeze_method_index, 0x2195);
    assert_eq!(shape.builder_size_method_index, 0x110a);
    assert_eq!(shape.builder_get_method_index, 0x1116);
    assert_eq!(shape.math_abs_method_index, 0x5989);
    assert_eq!(shape.contiguity_string_index, 0x2f10);
    assert_eq!(shape.exception_constructor_method_index, 0x5954);
    assert!(detect_rounded_polygon_constructor_shape(&wrong_code).is_none());
}

// cite: androidx RoundedPolygon.kt#transformed
#[test]
fn detects_rounded_polygon_transform_from_recorded_androidx_graphics_shapes_dex_shape() {
    let code = DexCodeItem {
        registers_size: 9,
        ins_size: 2,
        outs_size: 4,
        tries_size: 0,
        debug_info_offset: 0,
        instructions: vec![
            0x7052, 0x4207, 0x7152, 0x4208, 0x2071, 0x6ac9, 0x0010, 0x000b, 0x3071, 0x509a, 0x0810,
            0x000b, 0x0071, 0x2199, 0x0000, 0x020c, 0x7354, 0x4206, 0x1072, 0x5c0a, 0x0003, 0x040a,
            0x0512, 0x4535, 0x0012, 0x2072, 0x5bff, 0x0053, 0x060c, 0x061f, 0x155e, 0x206e, 0x6909,
            0x0086, 0x060c, 0x206e, 0x110c, 0x0062, 0x05d8, 0x0105, 0xef28, 0x1071, 0x2195, 0x0002,
            0x080c, 0x2071, 0x50b3, 0x0010, 0x020a, 0x2071, 0x50b4, 0x0010, 0x000a, 0x0122, 0x1567,
            0x4070, 0x6921, 0x0281, 0x0111,
        ],
        try_blocks: Vec::new(),
    };
    let mut wrong_code = code.clone();
    wrong_code.instructions[31] = 0x106e;

    let shape = detect_rounded_polygon_transform_shape(&code).expect("rounded polygon transform");

    assert_eq!(shape.center_x_field_index, 0x4207);
    assert_eq!(shape.center_y_field_index, 0x4208);
    assert_eq!(shape.float_pack_method_index, 0x6ac9);
    assert_eq!(shape.packed_point_transform_method_index, 0x509a);
    assert_eq!(shape.list_factory_method_index, 0x2199);
    assert_eq!(shape.features_field_index, 0x4206);
    assert_eq!(shape.list_size_method_index, 0x5c0a);
    assert_eq!(shape.list_get_method_index, 0x5bff);
    assert_eq!(shape.feature_type_index, 0x155e);
    assert_eq!(shape.feature_transform_method_index, 0x6909);
    assert_eq!(shape.list_add_method_index, 0x110c);
    assert_eq!(shape.list_freeze_method_index, 0x2195);
    assert_eq!(shape.center_x_method_index, 0x50b3);
    assert_eq!(shape.center_y_method_index, 0x50b4);
    assert_eq!(shape.polygon_type_index, 0x1567);
    assert_eq!(shape.constructor_method_index, 0x6921);
    assert!(detect_rounded_polygon_transform_shape(&wrong_code).is_none());
}

// cite: androidx RoundedPolygon.kt#calculateBounds
// cite: androidx Cubic.kt#calculateBounds, zeroLength
#[test]
fn detects_rounded_polygon_bounds_from_recorded_androidx_graphics_shapes_dex_shape() {
    let instructions = "1012 01dd 0113 4212 0138 0005 2123 19ae 0328 0108 0012 0174 59b9 0011 1321 2334 00db 0308 0011 3354 4209 106e 0d1f 0003 040a 1512 0614 ffff 7f7f 0712 6801 8901 7a01 5601 2b12 3c12 4a35 00b7 206e 1116 00a3 0d0c 0d1f 155a 106e 59b9 000d 106e 68fe 000d 0e0a df54 41e8 0e38 001b 0d44 070f 0d4b 0701 0d44 000f 0d4b 0001 0d44 070f 0d4b 0b01 0d44 000f 0d4b 0c01 1002 0000 1302 0002 1102 0007 1202 000b 6a28 0e44 070f 1002 0000 106e 68f8 000d 000a 2071 599e 000e 000a 0e44 100f 1302 0002 106e 68f9 000d 020a 2071 599e 002e 020a 0e44 070f 1102 0007 106e 68f8 000d 070a 2071 599a 007e 070a 0e44 100f 106e 68f9 000d 0d0a 2071 599a 00de 0d0a 0e44 0b0f 1202 000b 0b44 130f 2071 599e 00be 0b0a 2071 599e 00b0 000a 004b 1101 0044 0c0f 5b12 0e44 0b0f 2071 599e 00e0 000a 2071 599e 0002 000a 004b 1001 0044 120f 0244 130f 2071 599a 0020 000a 2071 599a 0007 000a 004b 1201 0044 0c0f 0244 0b0f 2071 599a 0020 000a 2071 599a 000d 000a 004b 0c01 0044 1101 2071 599e 0008 080a 0044 1001 2071 599e 0009 090a 0044 1201 2071 599a 0005 050a 0044 0c01 2071 599a 0006 060a 0ad8 010a 0702 0011 0202 0013 0002 0010 0029 ff49 1002 0000 1102 0007 1202 000b 084b 1101 094b 1001 054b 1201 064b 0c01 0111 0022 11b1 011a 2edd 2070 5954 0010 0027";
    let code = DexCodeItem {
        registers_size: 20,
        ins_size: 3,
        outs_size: 2,
        tries_size: 0,
        debug_info_offset: 0,
        instructions: instructions
            .split_whitespace()
            .map(|word| u16::from_str_radix(word, 16).expect("hex instruction"))
            .collect(),
        try_blocks: Vec::new(),
    };
    let mut wrong_code = code.clone();
    wrong_code.instructions[88] = 0x1071;

    let shape = detect_rounded_polygon_bounds_shape(&code).expect("rounded polygon bounds");

    assert_eq!(shape.float_array_type_index, 0x19ae);
    assert_eq!(shape.get_class_method_index, 0x59b9);
    assert_eq!(shape.cubics_field_index, 0x4209);
    assert_eq!(shape.builder_size_method_index, 0x0d1f);
    assert_eq!(shape.builder_get_method_index, 0x1116);
    assert_eq!(shape.cubic_type_index, 0x155a);
    assert_eq!(shape.cubic_zero_length_method_index, 0x68fe);
    assert_eq!(shape.cubic_points_field_index, 0x41e8);
    assert_eq!(shape.cubic_anchor_x_method_index, 0x68f8);
    assert_eq!(shape.cubic_anchor_y_method_index, 0x68f9);
    assert_eq!(shape.math_min_method_index, 0x599e);
    assert_eq!(shape.math_max_method_index, 0x599a);
    assert_eq!(shape.exception_type_index, 0x11b1);
    assert_eq!(shape.required_bounds_string_index, 0x2edd);
    assert_eq!(shape.exception_constructor_method_index, 0x5954);
    assert!(detect_rounded_polygon_bounds_shape(&wrong_code).is_none());
}

// cite: androidx RoundedPolygon.kt#normalized
#[test]
fn detects_rounded_polygon_normalized_from_recorded_androidx_graphics_shapes_dex_shape() {
    let code = DexCodeItem {
        registers_size: 8,
        ins_size: 1,
        outs_size: 4,
        tries_size: 0,
        debug_info_offset: 0,
        instructions: vec![
            0x0012, 0x3112, 0x3071, 0x6922, 0x0107, 0x000c, 0x2212, 0x0344, 0x0200, 0x0412, 0x0544,
            0x0400, 0x53c7, 0x0144, 0x0100, 0x1512, 0x0644, 0x0500, 0x61c7, 0x2071, 0x599a, 0x0013,
            0x060a, 0x03a7, 0x0306, 0x2282, 0x23c9, 0x0444, 0x0400, 0x43c7, 0x01a7, 0x0106, 0x21c9,
            0x0044, 0x0500, 0x01c7, 0x0022, 0x1566, 0x4070, 0x691e, 0x1630, 0x206e, 0x6924, 0x0007,
            0x000c, 0x0011,
        ],
        try_blocks: Vec::new(),
    };
    let mut wrong_code = code.clone();
    wrong_code.instructions[19] = 0x1071;

    let shape = detect_rounded_polygon_normalized_shape(&code).expect("rounded polygon normalized");

    assert_eq!(shape.bounds_method_index, 0x6922);
    assert_eq!(shape.math_max_method_index, 0x599a);
    assert_eq!(shape.transformer_type_index, 0x1566);
    assert_eq!(shape.transformer_constructor_method_index, 0x691e);
    assert_eq!(shape.transform_method_index, 0x6924);
    assert!(detect_rounded_polygon_normalized_shape(&wrong_code).is_none());
}

#[test]
fn measured_cubic_cut_intrinsic_allocates_recorded_graphics_shapes_objects() {
    let dex_file = crate::dex::parse_dex_file("classes.dex", minimal_measured_cubic_fields_dex())
        .expect("DEX should parse");
    let program = DexProgram::new(vec![dex_file]);
    let receiver = program.allocate_object("Lcom/example/Measured;".to_owned());
    let owner = program.allocate_object("Lcom/example/Owner;".to_owned());
    let center = program.allocate_object("Lcom/example/Center;".to_owned());
    let cubic = program.allocate_object("Lcom/example/Cubic;".to_owned());
    let values = [1.0, 0.0, 1.0, 0.55, 0.55, 1.0, 0.0, 1.0];
    let points = program.allocate_filled_array(
        "[F".to_owned(),
        values.into_iter().map(DexValue::Float).collect(),
    );
    let cubic_field = program.field_id(0, 0).expect("cubic field");
    let size_field = program.field_id(0, 1).expect("size field");
    let start_field = program.field_id(0, 2).expect("start field");
    let end_field = program.field_id(0, 3).expect("end field");
    let owner_field = program.field_id(0, 4).expect("owner field");
    let center_field = program.field_id(0, 5).expect("center field");
    let center_x_field = program.field_id(0, 6).expect("center x field");
    let center_y_field = program.field_id(0, 7).expect("center y field");
    let points_field = program.field_id(0, 8).expect("points field");
    let pair_first_field = program.field_id(0, 9).expect("pair first field");
    let pair_second_field = program.field_id(0, 10).expect("pair second field");
    assert!(program.write_stored_instance_field(&center, &center_x_field, DexValue::Float(0.0)));
    assert!(program.write_stored_instance_field(&center, &center_y_field, DexValue::Float(0.0)));
    assert!(program.write_stored_instance_field(&owner, &center_field, center.clone()));
    assert!(program.write_stored_instance_field(&cubic, &points_field, points));
    assert!(program.write_stored_instance_field(&receiver, &cubic_field, cubic));
    assert!(program.write_stored_instance_field(
        &receiver,
        &size_field,
        DexValue::Float(measured_cubic_size(&values, 0.0, 0.0))
    ));
    assert!(program.write_stored_instance_field(&receiver, &start_field, DexValue::Float(0.0)));
    assert!(program.write_stored_instance_field(&receiver, &end_field, DexValue::Float(1.0)));
    assert!(program.write_stored_instance_field(&receiver, &owner_field, owner.clone()));
    let resolved = ResolvedMethod {
        dex_file_index: 0,
        dex_file_name: "classes.dex".to_owned(),
        class_descriptor: "Lcom/example/Measured;".to_owned(),
        encoded_method: DexEncodedMethod {
            method_index: 0,
            access_flags: 0,
            code_offset: 1,
            kind: crate::dex::DexMethodKind::Virtual,
        },
        method: DexMethodId {
            class_descriptor: "Lcom/example/Measured;".to_owned(),
            name: "cut".to_owned(),
            proto: crate::dex::DexProto {
                shorty: "LF".to_owned(),
                return_type: "Lcom/example/Pair;".to_owned(),
                parameters: vec!["F".to_owned()],
            },
        },
    };
    let handler = DexRunInvocationHandler {
        current_dex_file_index: 0,
        current_class_descriptor: None,
        program: &program,
        depth: 0,
    };
    let invocation = test_invocation(vec![receiver, DexValue::Float(0.5)]);

    let result = handler
        .apply_method_intrinsic(
            &resolved,
            MethodIntrinsic::MeasuredCubicCut {
                receiver_arg: 0,
                progress_arg: 1,
                cubic_field_index: 0,
                size_field_index: 1,
                start_field_index: 2,
                end_field_index: 3,
                owner_field_index: 4,
                owner_center_field_index: 5,
                center_x_field_index: 6,
                center_y_field_index: 7,
                cubic_points_field_index: 8,
                pair_first_field_index: 9,
                pair_second_field_index: 10,
            },
            &invocation,
        )
        .expect("intrinsic result");
    let Some(DexInvocationResult::Value(pair)) = result else {
        panic!("expected pair result");
    };
    let left = program
        .read_stored_instance_field(&pair, &pair_first_field)
        .expect("left measured cubic");
    let right = program
        .read_stored_instance_field(&pair, &pair_second_field)
        .expect("right measured cubic");
    let left_cubic = program
        .read_stored_instance_field(&left, &cubic_field)
        .expect("left cubic");
    let right_cubic = program
        .read_stored_instance_field(&right, &cubic_field)
        .expect("right cubic");
    let cut = measured_cubic_cut_t(
        &values,
        0.0,
        0.0,
        graphics_vector_angle(values[0], values[1]),
        measured_cubic_size(&values, 0.0, 0.0) * 0.5,
    );
    let (expected_left, expected_right) = split_cubic_values(&values, cut);
    let read_points = |object: &DexValue| {
        let array = program
            .read_stored_instance_field(object, &points_field)
            .expect("points array");
        let key = map_identity_key(&array);
        program
            .runtime
            .borrow()
            .array_entries
            .get(&key)
            .expect("array entries")
            .clone()
    };

    assert_eq!(
        program.read_stored_instance_field(&left, &owner_field),
        Some(owner.clone())
    );
    assert_eq!(
        program.read_stored_instance_field(&right, &owner_field),
        Some(owner)
    );
    assert_eq!(
        program.read_stored_instance_field(&left, &start_field),
        Some(DexValue::Float(0.0))
    );
    assert_eq!(
        program.read_stored_instance_field(&left, &end_field),
        Some(DexValue::Float(0.5))
    );
    assert_eq!(
        program.read_stored_instance_field(&right, &start_field),
        Some(DexValue::Float(0.5))
    );
    assert_eq!(
        program.read_stored_instance_field(&right, &end_field),
        Some(DexValue::Float(1.0))
    );
    assert_eq!(
        read_points(&left_cubic),
        expected_left
            .into_iter()
            .map(|value| DexValue::Int(value.to_bits() as i32))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        read_points(&right_cubic),
        expected_right
            .into_iter()
            .map(|value| DexValue::Int(value.to_bits() as i32))
            .collect::<Vec<_>>()
    );
}

// cite: androidx FeatureMapping.kt#featureDistSquared, featureRepresentativePoint
// cite: androidx Point.kt#getDistanceSquared
#[test]
fn feature_distance_squared_intrinsic_matches_androidx_feature_mapping() {
    let dex_file = crate::dex::parse_dex_file("classes.dex", minimal_feature_distance_fields_dex())
        .expect("DEX should parse");
    let program = DexProgram::new(vec![dex_file]);
    let cubics_field = program.field_id(0, 0).expect("cubics field");
    let points_field = program.field_id(0, 1).expect("points field");
    let convex_field = program.field_id(0, 2).expect("convex field");
    let handler = DexRunInvocationHandler {
        current_dex_file_index: 0,
        current_class_descriptor: None,
        program: &program,
        depth: 0,
    };
    let resolved = ResolvedMethod {
        dex_file_index: 0,
        dex_file_name: "classes.dex".to_owned(),
        class_descriptor: "Lcom/example/FeatureMapping;".to_owned(),
        encoded_method: DexEncodedMethod {
            method_index: 0,
            access_flags: DEX_ACC_STATIC,
            code_offset: 1,
            kind: crate::dex::DexMethodKind::Direct,
        },
        method: DexMethodId {
            class_descriptor: "Lcom/example/FeatureMapping;".to_owned(),
            name: "featureDistSquared".to_owned(),
            proto: crate::dex::DexProto {
                shorty: "FLL".to_owned(),
                return_type: "F".to_owned(),
                parameters: vec![
                    "Lcom/example/Feature;".to_owned(),
                    "Lcom/example/Feature;".to_owned(),
                ],
            },
        },
    };
    let make_cubic = |values: [f32; 8]| {
        let cubic = program.allocate_object("Lcom/example/Cubic;".to_owned());
        let points = program.allocate_filled_array(
            "[F".to_owned(),
            values.into_iter().map(DexValue::Float).collect(),
        );
        assert!(program.write_stored_instance_field(&cubic, &points_field, points));
        cubic
    };
    let make_feature = |class_descriptor: &str, cubics: Vec<DexValue>, convex: Option<bool>| {
        let feature = program.allocate_object(class_descriptor.to_owned());
        let list = program.allocate_object("Ljava/util/ArrayList;".to_owned());
        program
            .runtime
            .borrow_mut()
            .list_entries
            .insert(map_identity_key(&list), cubics);
        assert!(program.write_stored_instance_field(&feature, &cubics_field, list));
        if let Some(convex) = convex {
            assert!(program.write_stored_instance_field(
                &feature,
                &convex_field,
                DexValue::Int(i32::from(convex))
            ));
        }
        feature
    };

    let left = make_feature(
        "Lcom/example/Feature;",
        vec![make_cubic([0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 2.0, 0.0])],
        None,
    );
    let right = make_feature(
        "Lcom/example/Feature;",
        vec![make_cubic([0.0, 2.0, 0.0, 2.0, 0.0, 2.0, 2.0, 2.0])],
        None,
    );
    let result = handler
        .apply_method_intrinsic(
            &resolved,
            MethodIntrinsic::FeatureDistanceSquared {
                left_arg: 0,
                right_arg: 1,
                corner_type_index: 1,
                convex_field_index: 2,
                cubics_field_index: 0,
                cubic_points_field_index: 1,
            },
            &test_invocation(vec![left, right]),
        )
        .expect("intrinsic result");

    assert_eq!(
        result,
        Some(DexInvocationResult::Value(DexValue::Float(4.0)))
    );

    let convex = make_feature(
        "Lcom/example/Corner;",
        vec![make_cubic([0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 2.0, 0.0])],
        Some(true),
    );
    let concave = make_feature(
        "Lcom/example/Corner;",
        vec![make_cubic([0.0, 2.0, 0.0, 2.0, 0.0, 2.0, 2.0, 2.0])],
        Some(false),
    );
    let result = handler
        .apply_method_intrinsic(
            &resolved,
            MethodIntrinsic::FeatureDistanceSquared {
                left_arg: 0,
                right_arg: 1,
                corner_type_index: 1,
                convex_field_index: 2,
                cubics_field_index: 0,
                cubic_points_field_index: 1,
            },
            &test_invocation(vec![convex, concave]),
        )
        .expect("intrinsic result");

    assert_eq!(
        result,
        Some(DexInvocationResult::Value(DexValue::Float(f32::MAX)))
    );
}

// cite: libcore Arrays.java#copyOf
// cite: libcore System.java#arraycopy
#[test]
fn object_array_gap_insert_intrinsic_grows_and_shifts_entries() {
    let dex_file = crate::dex::parse_dex_file("classes.dex", minimal_field_class_dex())
        .expect("DEX should parse");
    let program = DexProgram::new(vec![dex_file]);
    let receiver = program.allocate_object("Lcom/example/Test;".to_owned());
    let first = DexValue::String(std::rc::Rc::from("first"));
    let second = DexValue::String(std::rc::Rc::from("second"));
    let array = program.allocate_filled_array(
        "[Ljava/lang/Object;".to_owned(),
        vec![first.clone(), second.clone()],
    );
    let size_field = program.field_id(0, 0).expect("size field");
    let array_field = program.field_id(0, 1).expect("array field");
    assert!(program.write_stored_instance_field(&receiver, &size_field, DexValue::Int(2)));
    assert!(program.write_stored_instance_field(&receiver, &array_field, array));

    let resolved = ResolvedMethod {
        dex_file_index: 0,
        dex_file_name: "classes.dex".to_owned(),
        class_descriptor: "Lcom/example/Test;".to_owned(),
        encoded_method: DexEncodedMethod {
            method_index: 0,
            access_flags: 0,
            code_offset: 1,
            kind: crate::dex::DexMethodKind::Virtual,
        },
        method: DexMethodId {
            class_descriptor: "Lcom/example/Test;".to_owned(),
            name: "h".to_owned(),
            proto: crate::dex::DexProto {
                shorty: "VII".to_owned(),
                return_type: "V".to_owned(),
                parameters: vec!["I".to_owned(), "I".to_owned()],
            },
        },
    };
    let handler = DexRunInvocationHandler {
        current_dex_file_index: 0,
        current_class_descriptor: None,
        program: &program,
        depth: 0,
    };
    let invocation = test_invocation(vec![receiver.clone(), DexValue::Int(1), DexValue::Int(1)]);

    let result = handler
        .apply_method_intrinsic(
            &resolved,
            MethodIntrinsic::ObjectArrayGapInsert {
                receiver_arg: 0,
                index_arg: 1,
                count_arg: 2,
                array_field_index: 1,
                size_field_index: 0,
            },
            &invocation,
        )
        .expect("intrinsic result");

    assert_eq!(result, Some(DexInvocationResult::Void));
    assert_eq!(
        program.read_stored_instance_field(&receiver, &size_field),
        Some(DexValue::Int(3))
    );
    let stored_array = program
        .read_stored_instance_field(&receiver, &array_field)
        .expect("stored array");
    let stored_key = map_identity_key(&stored_array);
    assert_eq!(
        program.runtime.borrow().array_entries.get(&stored_key),
        Some(&vec![first, second.clone(), second])
    );
}

// cite: libcore AbstractList.java#modCount
// cite: libcore Arrays.java#copyOf
#[test]
fn object_array_append_intrinsic_grows_writes_size_and_mod_count() {
    let dex_file =
        crate::dex::parse_dex_file("classes.dex", minimal_list_builder_field_class_dex())
            .expect("DEX should parse");
    let program = DexProgram::new(vec![dex_file]);
    let receiver = program.allocate_object("Lcom/example/ListBuilder;".to_owned());
    let first = DexValue::String(std::rc::Rc::from("first"));
    let second = DexValue::String(std::rc::Rc::from("second"));
    let array =
        program.allocate_filled_array("[Ljava/lang/Object;".to_owned(), vec![first.clone()]);
    let array_field = program.field_id(0, 0).expect("array field");
    let size_field = program.field_id(0, 1).expect("size field");
    let read_only_field = program.field_id(0, 2).expect("read only field");
    let mod_count_field = program.field_id(0, 3).expect("mod count field");
    assert!(program.write_stored_instance_field(&receiver, &array_field, array));
    assert!(program.write_stored_instance_field(&receiver, &size_field, DexValue::Int(1)));
    assert!(program.write_stored_instance_field(&receiver, &read_only_field, DexValue::Int(0)));
    assert!(program.write_stored_instance_field(&receiver, &mod_count_field, DexValue::Int(7)));
    let resolved = ResolvedMethod {
        dex_file_index: 0,
        dex_file_name: "classes.dex".to_owned(),
        class_descriptor: "Lcom/example/ListBuilder;".to_owned(),
        encoded_method: DexEncodedMethod {
            method_index: 0,
            access_flags: 0,
            code_offset: 1,
            kind: crate::dex::DexMethodKind::Virtual,
        },
        method: DexMethodId {
            class_descriptor: "Lcom/example/ListBuilder;".to_owned(),
            name: "add".to_owned(),
            proto: crate::dex::DexProto {
                shorty: "ZL".to_owned(),
                return_type: "Z".to_owned(),
                parameters: vec!["Ljava/lang/Object;".to_owned()],
            },
        },
    };
    let handler = DexRunInvocationHandler {
        current_dex_file_index: 0,
        current_class_descriptor: None,
        program: &program,
        depth: 0,
    };
    let invocation = test_invocation(vec![receiver.clone(), second.clone()]);

    let result = handler
        .apply_method_intrinsic(
            &resolved,
            MethodIntrinsic::ObjectArrayAppend {
                receiver_arg: 0,
                value_arg: 1,
                array_field_index: 0,
                size_field_index: 1,
                read_only_field_index: 2,
                mod_count_field_index: 3,
            },
            &invocation,
        )
        .expect("intrinsic result");

    assert_eq!(result, Some(DexInvocationResult::Value(DexValue::Int(1))));
    assert_eq!(
        program.read_stored_instance_field(&receiver, &size_field),
        Some(DexValue::Int(2))
    );
    assert_eq!(
        program.read_stored_instance_field(&receiver, &mod_count_field),
        Some(DexValue::Int(8))
    );
    let stored_array = program
        .read_stored_instance_field(&receiver, &array_field)
        .expect("stored array");
    let stored_key = map_identity_key(&stored_array);
    assert_eq!(
        program.runtime.borrow().array_entries.get(&stored_key),
        Some(&vec![first, second])
    );
}

// cite: libcore System.java#arraycopy
#[test]
fn slot_object_gap_ensure_intrinsic_moves_grows_and_updates_anchor_fields() {
    let dex_file = crate::dex::parse_dex_file("classes.dex", minimal_slot_gap_field_class_dex())
        .expect("DEX should parse");
    let program = DexProgram::new(vec![dex_file]);
    let receiver = program.allocate_object("Lcom/example/Slot;".to_owned());
    let first = DexValue::String(std::rc::Rc::from("first"));
    let second = DexValue::String(std::rc::Rc::from("second"));
    let third = DexValue::String(std::rc::Rc::from("third"));
    let fourth = DexValue::String(std::rc::Rc::from("fourth"));
    let object_array = program.allocate_filled_array(
        "[Ljava/lang/Object;".to_owned(),
        vec![
            first.clone(),
            second.clone(),
            DexValue::Null,
            third.clone(),
            fourth.clone(),
        ],
    );
    let mut group_values = vec![DexValue::Int(0); 15];
    group_values[4] = DexValue::Int(-5);
    group_values[9] = DexValue::Int(-4);
    let group_array = program.allocate_filled_array("[I".to_owned(), group_values);
    let slot_index_field = program.field_id(0, 0).expect("slot index field");
    let slot_limit_field = program.field_id(0, 1).expect("slot limit field");
    let object_array_field = program.field_id(0, 2).expect("object array field");
    let object_gap_start_field = program.field_id(0, 3).expect("object gap start field");
    let object_gap_size_field = program.field_id(0, 4).expect("object gap size field");
    let object_anchor_field = program.field_id(0, 5).expect("object anchor field");
    let group_array_field = program.field_id(0, 6).expect("group array field");
    let group_gap_start_field = program.field_id(0, 7).expect("group gap start field");
    let group_gap_size_field = program.field_id(0, 8).expect("group gap size field");
    assert!(program.write_stored_instance_field(&receiver, &slot_index_field, DexValue::Int(1)));
    assert!(program.write_stored_instance_field(&receiver, &slot_limit_field, DexValue::Int(4)));
    assert!(program.write_stored_instance_field(&receiver, &object_array_field, object_array));
    assert!(program.write_stored_instance_field(
        &receiver,
        &object_gap_start_field,
        DexValue::Int(2)
    ));
    assert!(program.write_stored_instance_field(
        &receiver,
        &object_gap_size_field,
        DexValue::Int(1)
    ));
    assert!(program.write_stored_instance_field(&receiver, &object_anchor_field, DexValue::Int(0)));
    assert!(program.write_stored_instance_field(&receiver, &group_array_field, group_array));
    assert!(program.write_stored_instance_field(
        &receiver,
        &group_gap_start_field,
        DexValue::Int(3)
    ));
    assert!(program.write_stored_instance_field(
        &receiver,
        &group_gap_size_field,
        DexValue::Int(0)
    ));
    let resolved = ResolvedMethod {
        dex_file_index: 0,
        dex_file_name: "classes.dex".to_owned(),
        class_descriptor: "Lcom/example/Slot;".to_owned(),
        encoded_method: DexEncodedMethod {
            method_index: 0,
            access_flags: 0,
            code_offset: 1,
            kind: crate::dex::DexMethodKind::Virtual,
        },
        method: DexMethodId {
            class_descriptor: "Lcom/example/Slot;".to_owned(),
            name: "v".to_owned(),
            proto: crate::dex::DexProto {
                shorty: "VII".to_owned(),
                return_type: "V".to_owned(),
                parameters: vec!["I".to_owned(), "I".to_owned()],
            },
        },
    };
    let handler = DexRunInvocationHandler {
        current_dex_file_index: 0,
        current_class_descriptor: None,
        program: &program,
        depth: 0,
    };
    let invocation = test_invocation(vec![receiver.clone(), DexValue::Int(2), DexValue::Int(1)]);

    let result = handler
        .apply_method_intrinsic(
            &resolved,
            MethodIntrinsic::SlotObjectGapEnsure {
                receiver_arg: 0,
                count_arg: 1,
                anchor_arg: 2,
                slot_index_field_index: 0,
                slot_limit_field_index: 1,
                object_array_field_index: 2,
                object_gap_start_field_index: 3,
                object_gap_size_field_index: 4,
                object_anchor_field_index: 5,
                group_array_field_index: 6,
                group_gap_start_field_index: 7,
                group_gap_size_field_index: 8,
            },
            &invocation,
        )
        .expect("intrinsic result");

    assert_eq!(result, Some(DexInvocationResult::Void));
    assert_eq!(
        program.read_stored_instance_field(&receiver, &slot_limit_field),
        Some(DexValue::Int(6))
    );
    assert_eq!(
        program.read_stored_instance_field(&receiver, &object_gap_start_field),
        Some(DexValue::Int(3))
    );
    assert_eq!(
        program.read_stored_instance_field(&receiver, &object_gap_size_field),
        Some(DexValue::Int(26))
    );
    assert_eq!(
        program.read_stored_instance_field(&receiver, &object_anchor_field),
        Some(DexValue::Int(2))
    );
    let stored_object_array = program
        .read_stored_instance_field(&receiver, &object_array_field)
        .expect("stored object array");
    let stored_group_array = program
        .read_stored_instance_field(&receiver, &group_array_field)
        .expect("stored group array");
    let object_key = map_identity_key(&stored_object_array);
    let group_key = map_identity_key(&stored_group_array);
    let runtime = program.runtime.borrow();
    let object_values = runtime
        .array_entries
        .get(&object_key)
        .expect("object array values");
    let group_values = runtime
        .array_entries
        .get(&group_key)
        .expect("group array values");
    assert_eq!(object_values.len(), 32);
    assert_eq!(object_values[0], first);
    assert_eq!(object_values[1], DexValue::Null);
    assert_eq!(object_values[2], DexValue::Null);
    assert_eq!(object_values[29], second);
    assert_eq!(object_values[30], third);
    assert_eq!(object_values[31], fourth);
    assert_eq!(group_values[4], DexValue::Int(0));
    assert_eq!(group_values[9], DexValue::Int(1));
}

// cite: androidx SlotTable.kt#update, rawUpdate, skip, set
#[test]
fn slot_object_update_intrinsic_replaces_current_slot_and_returns_previous_value() {
    let dex_file = crate::dex::parse_dex_file("classes.dex", minimal_slot_gap_field_class_dex())
        .expect("DEX should parse");
    let program = DexProgram::new(vec![dex_file]);
    let receiver = program.allocate_object("Lcom/example/Slot;".to_owned());
    let first = DexValue::String(std::rc::Rc::from("first"));
    let old = DexValue::String(std::rc::Rc::from("old"));
    let third = DexValue::String(std::rc::Rc::from("third"));
    let new = DexValue::String(std::rc::Rc::from("new"));
    let object_array = program.allocate_filled_array(
        "[Ljava/lang/Object;".to_owned(),
        vec![first.clone(), old.clone(), third.clone()],
    );
    let group_array = program.allocate_filled_array("[I".to_owned(), vec![DexValue::Int(0); 5]);
    let slot_index_field = program.field_id(0, 0).expect("slot index field");
    let slot_limit_field = program.field_id(0, 1).expect("slot limit field");
    let object_array_field = program.field_id(0, 2).expect("object array field");
    let object_gap_start_field = program.field_id(0, 3).expect("object gap start field");
    let object_gap_size_field = program.field_id(0, 4).expect("object gap size field");
    let object_anchor_field = program.field_id(0, 5).expect("object anchor field");
    let group_array_field = program.field_id(0, 6).expect("group array field");
    let group_gap_start_field = program.field_id(0, 7).expect("group gap start field");
    let group_gap_size_field = program.field_id(0, 8).expect("group gap size field");
    let insert_count_field = program.field_id(0, 9).expect("insert count field");
    let insert_anchor_field = program.field_id(0, 10).expect("insert anchor field");
    assert!(program.write_stored_instance_field(&receiver, &slot_index_field, DexValue::Int(1)));
    assert!(program.write_stored_instance_field(&receiver, &slot_limit_field, DexValue::Int(3)));
    assert!(program.write_stored_instance_field(&receiver, &object_array_field, object_array));
    assert!(program.write_stored_instance_field(
        &receiver,
        &object_gap_start_field,
        DexValue::Int(3)
    ));
    assert!(program.write_stored_instance_field(
        &receiver,
        &object_gap_size_field,
        DexValue::Int(0)
    ));
    assert!(program.write_stored_instance_field(&receiver, &object_anchor_field, DexValue::Int(1)));
    assert!(program.write_stored_instance_field(&receiver, &group_array_field, group_array));
    assert!(program.write_stored_instance_field(
        &receiver,
        &group_gap_start_field,
        DexValue::Int(1)
    ));
    assert!(program.write_stored_instance_field(
        &receiver,
        &group_gap_size_field,
        DexValue::Int(0)
    ));
    assert!(program.write_stored_instance_field(&receiver, &insert_count_field, DexValue::Int(0)));
    assert!(program.write_stored_instance_field(&receiver, &insert_anchor_field, DexValue::Int(0)));
    let resolved = ResolvedMethod {
        dex_file_index: 0,
        dex_file_name: "classes.dex".to_owned(),
        class_descriptor: "Lcom/example/Slot;".to_owned(),
        encoded_method: DexEncodedMethod {
            method_index: 0,
            access_flags: 0,
            code_offset: 1,
            kind: crate::dex::DexMethodKind::Virtual,
        },
        method: DexMethodId {
            class_descriptor: "Lcom/example/Slot;".to_owned(),
            name: "update".to_owned(),
            proto: crate::dex::DexProto {
                shorty: "LL".to_owned(),
                return_type: "Ljava/lang/Object;".to_owned(),
                parameters: vec!["Ljava/lang/Object;".to_owned()],
            },
        },
    };
    let handler = DexRunInvocationHandler {
        current_dex_file_index: 0,
        current_class_descriptor: None,
        program: &program,
        depth: 0,
    };
    let invocation = test_invocation(vec![receiver.clone(), new.clone()]);

    let result = handler
        .apply_method_intrinsic(
            &resolved,
            MethodIntrinsic::SlotObjectUpdate {
                receiver_arg: 0,
                value_arg: 1,
                insert_count_field_index: 9,
                insert_anchor_field_index: 10,
                slot_index_field_index: 0,
                slot_limit_field_index: 1,
                object_array_field_index: 2,
                object_gap_start_field_index: 3,
                object_gap_size_field_index: 4,
                object_anchor_field_index: 5,
                group_array_field_index: 6,
                group_gap_start_field_index: 7,
                group_gap_size_field_index: 8,
            },
            &invocation,
        )
        .expect("intrinsic result");

    assert_eq!(result, Some(DexInvocationResult::Value(old)));
    assert_eq!(
        program.read_stored_instance_field(&receiver, &slot_index_field),
        Some(DexValue::Int(2))
    );
    let stored_object_array = program
        .read_stored_instance_field(&receiver, &object_array_field)
        .expect("stored object array");
    let object_key = map_identity_key(&stored_object_array);
    assert_eq!(
        program.runtime.borrow().array_entries.get(&object_key),
        Some(&vec![first, new, third])
    );
}

// cite: androidx SlotTable.kt#update, rawUpdate, skip, set, insertSlots, moveSlotGapTo
#[test]
fn slot_object_update_intrinsic_inserts_grows_gap_and_writes_new_slot() {
    let dex_file = crate::dex::parse_dex_file("classes.dex", minimal_slot_gap_field_class_dex())
        .expect("DEX should parse");
    let program = DexProgram::new(vec![dex_file]);
    let receiver = program.allocate_object("Lcom/example/Slot;".to_owned());
    let first = DexValue::String(std::rc::Rc::from("first"));
    let second = DexValue::String(std::rc::Rc::from("second"));
    let new = DexValue::String(std::rc::Rc::from("new"));
    let object_array = program.allocate_filled_array(
        "[Ljava/lang/Object;".to_owned(),
        vec![first.clone(), second.clone()],
    );
    let group_array = program.allocate_filled_array("[I".to_owned(), vec![DexValue::Int(0); 5]);
    let slot_index_field = program.field_id(0, 0).expect("slot index field");
    let slot_limit_field = program.field_id(0, 1).expect("slot limit field");
    let object_array_field = program.field_id(0, 2).expect("object array field");
    let object_gap_start_field = program.field_id(0, 3).expect("object gap start field");
    let object_gap_size_field = program.field_id(0, 4).expect("object gap size field");
    let object_anchor_field = program.field_id(0, 5).expect("object anchor field");
    let group_array_field = program.field_id(0, 6).expect("group array field");
    let group_gap_start_field = program.field_id(0, 7).expect("group gap start field");
    let group_gap_size_field = program.field_id(0, 8).expect("group gap size field");
    let insert_count_field = program.field_id(0, 9).expect("insert count field");
    let insert_anchor_field = program.field_id(0, 10).expect("insert anchor field");
    assert!(program.write_stored_instance_field(&receiver, &slot_index_field, DexValue::Int(1)));
    assert!(program.write_stored_instance_field(&receiver, &slot_limit_field, DexValue::Int(2)));
    assert!(program.write_stored_instance_field(&receiver, &object_array_field, object_array));
    assert!(program.write_stored_instance_field(
        &receiver,
        &object_gap_start_field,
        DexValue::Int(1)
    ));
    assert!(program.write_stored_instance_field(
        &receiver,
        &object_gap_size_field,
        DexValue::Int(0)
    ));
    assert!(program.write_stored_instance_field(&receiver, &object_anchor_field, DexValue::Int(1)));
    assert!(program.write_stored_instance_field(&receiver, &group_array_field, group_array));
    assert!(program.write_stored_instance_field(
        &receiver,
        &group_gap_start_field,
        DexValue::Int(1)
    ));
    assert!(program.write_stored_instance_field(
        &receiver,
        &group_gap_size_field,
        DexValue::Int(0)
    ));
    assert!(program.write_stored_instance_field(&receiver, &insert_count_field, DexValue::Int(1)));
    assert!(program.write_stored_instance_field(&receiver, &insert_anchor_field, DexValue::Int(0)));
    let resolved = ResolvedMethod {
        dex_file_index: 0,
        dex_file_name: "classes.dex".to_owned(),
        class_descriptor: "Lcom/example/Slot;".to_owned(),
        encoded_method: DexEncodedMethod {
            method_index: 0,
            access_flags: 0,
            code_offset: 1,
            kind: crate::dex::DexMethodKind::Virtual,
        },
        method: DexMethodId {
            class_descriptor: "Lcom/example/Slot;".to_owned(),
            name: "update".to_owned(),
            proto: crate::dex::DexProto {
                shorty: "LL".to_owned(),
                return_type: "Ljava/lang/Object;".to_owned(),
                parameters: vec!["Ljava/lang/Object;".to_owned()],
            },
        },
    };
    let handler = DexRunInvocationHandler {
        current_dex_file_index: 0,
        current_class_descriptor: None,
        program: &program,
        depth: 0,
    };
    let invocation = test_invocation(vec![receiver.clone(), new.clone()]);

    let result = handler
        .apply_method_intrinsic(
            &resolved,
            MethodIntrinsic::SlotObjectUpdate {
                receiver_arg: 0,
                value_arg: 1,
                insert_count_field_index: 9,
                insert_anchor_field_index: 10,
                slot_index_field_index: 0,
                slot_limit_field_index: 1,
                object_array_field_index: 2,
                object_gap_start_field_index: 3,
                object_gap_size_field_index: 4,
                object_anchor_field_index: 5,
                group_array_field_index: 6,
                group_gap_start_field_index: 7,
                group_gap_size_field_index: 8,
            },
            &invocation,
        )
        .expect("intrinsic result");

    assert_eq!(result, Some(DexInvocationResult::Value(DexValue::Null)));
    assert_eq!(
        program.read_stored_instance_field(&receiver, &slot_index_field),
        Some(DexValue::Int(2))
    );
    assert_eq!(
        program.read_stored_instance_field(&receiver, &slot_limit_field),
        Some(DexValue::Int(3))
    );
    assert_eq!(
        program.read_stored_instance_field(&receiver, &object_gap_start_field),
        Some(DexValue::Int(2))
    );
    assert_eq!(
        program.read_stored_instance_field(&receiver, &object_gap_size_field),
        Some(DexValue::Int(29))
    );
    let stored_object_array = program
        .read_stored_instance_field(&receiver, &object_array_field)
        .expect("stored object array");
    let object_key = map_identity_key(&stored_object_array);
    let runtime = program.runtime.borrow();
    let object_values = runtime
        .array_entries
        .get(&object_key)
        .expect("object array values");
    assert_eq!(object_values.len(), 32);
    assert_eq!(object_values[0], first);
    assert_eq!(object_values[1], new);
    assert_eq!(object_values[31], second);
}

// cite: androidx SlotTable.kt#dataIndex, dataAnchorToDataIndex
#[test]
fn slot_group_data_index_intrinsic_decodes_positive_negative_and_end_anchors() {
    let dex_file = crate::dex::parse_dex_file("classes.dex", minimal_slot_gap_field_class_dex())
        .expect("DEX should parse");
    let program = DexProgram::new(vec![dex_file]);
    let receiver = program.allocate_object("Lcom/example/Slot;".to_owned());
    let object_array = program.allocate_filled_array(
        "[Ljava/lang/Object;".to_owned(),
        vec![
            DexValue::Null,
            DexValue::Null,
            DexValue::Null,
            DexValue::Null,
            DexValue::Null,
        ],
    );
    let mut group_values = vec![DexValue::Int(0); 15];
    group_values[4] = DexValue::Int(1);
    group_values[9] = DexValue::Int(-1);
    let group_array = program.allocate_filled_array("[I".to_owned(), group_values);
    let object_array_field = program.field_id(0, 2).expect("object array field");
    let object_gap_size_field = program.field_id(0, 4).expect("object gap size field");
    let group_array_field = program.field_id(0, 6).expect("group array field");
    assert!(program.write_stored_instance_field(&receiver, &object_array_field, object_array));
    assert!(program.write_stored_instance_field(
        &receiver,
        &object_gap_size_field,
        DexValue::Int(2)
    ));
    assert!(program.write_stored_instance_field(
        &receiver,
        &group_array_field,
        group_array.clone()
    ));
    let resolved = ResolvedMethod {
        dex_file_index: 0,
        dex_file_name: "classes.dex".to_owned(),
        class_descriptor: "Lcom/example/Slot;".to_owned(),
        encoded_method: DexEncodedMethod {
            method_index: 0,
            access_flags: 0,
            code_offset: 1,
            kind: crate::dex::DexMethodKind::Virtual,
        },
        method: DexMethodId {
            class_descriptor: "Lcom/example/Slot;".to_owned(),
            name: "dataIndex".to_owned(),
            proto: crate::dex::DexProto {
                shorty: "I[I".to_owned(),
                return_type: "I".to_owned(),
                parameters: vec!["[I".to_owned(), "I".to_owned()],
            },
        },
    };
    let handler = DexRunInvocationHandler {
        current_dex_file_index: 0,
        current_class_descriptor: None,
        program: &program,
        depth: 0,
    };
    let intrinsic = MethodIntrinsic::SlotGroupDataIndex {
        receiver_arg: 0,
        groups_arg: 1,
        address_arg: 2,
        group_array_field_index: 6,
        object_array_field_index: 2,
        object_gap_size_field_index: 4,
    };

    let positive = handler
        .apply_method_intrinsic(
            &resolved,
            intrinsic,
            &test_invocation(vec![
                receiver.clone(),
                group_array.clone(),
                DexValue::Int(0),
            ]),
        )
        .expect("positive result");
    let negative = handler
        .apply_method_intrinsic(
            &resolved,
            intrinsic,
            &test_invocation(vec![
                receiver.clone(),
                group_array.clone(),
                DexValue::Int(1),
            ]),
        )
        .expect("negative result");
    let end = handler
        .apply_method_intrinsic(
            &resolved,
            intrinsic,
            &test_invocation(vec![receiver, group_array, DexValue::Int(3)]),
        )
        .expect("end result");

    assert_eq!(positive, Some(DexInvocationResult::Value(DexValue::Int(1))));
    assert_eq!(negative, Some(DexInvocationResult::Value(DexValue::Int(3))));
    assert_eq!(end, Some(DexInvocationResult::Value(DexValue::Int(3))));
}

// cite: androidx SlotTable.kt#size, capacity, groupIndexToAddress
#[test]
fn slot_group_size_and_address_intrinsics_decode_group_gap_state() {
    let dex_file = crate::dex::parse_dex_file("classes.dex", minimal_slot_gap_field_class_dex())
        .expect("DEX should parse");
    let program = DexProgram::new(vec![dex_file]);
    let receiver = program.allocate_object("Lcom/example/Slot;".to_owned());
    let group_array = program.allocate_filled_array("[I".to_owned(), vec![DexValue::Int(0); 20]);
    let group_array_field = program.field_id(0, 6).expect("group array field");
    let group_gap_start_field = program.field_id(0, 7).expect("group gap start field");
    let group_gap_size_field = program.field_id(0, 8).expect("group gap size field");
    assert!(program.write_stored_instance_field(&receiver, &group_array_field, group_array));
    assert!(program.write_stored_instance_field(
        &receiver,
        &group_gap_start_field,
        DexValue::Int(2)
    ));
    assert!(program.write_stored_instance_field(
        &receiver,
        &group_gap_size_field,
        DexValue::Int(1)
    ));
    let resolved_size = ResolvedMethod {
        dex_file_index: 0,
        dex_file_name: "classes.dex".to_owned(),
        class_descriptor: "Lcom/example/Slot;".to_owned(),
        encoded_method: DexEncodedMethod {
            method_index: 0,
            access_flags: 0,
            code_offset: 1,
            kind: crate::dex::DexMethodKind::Virtual,
        },
        method: DexMethodId {
            class_descriptor: "Lcom/example/Slot;".to_owned(),
            name: "size".to_owned(),
            proto: crate::dex::DexProto {
                shorty: "I".to_owned(),
                return_type: "I".to_owned(),
                parameters: Vec::new(),
            },
        },
    };
    let resolved_address = ResolvedMethod {
        dex_file_index: 0,
        dex_file_name: "classes.dex".to_owned(),
        class_descriptor: "Lcom/example/Slot;".to_owned(),
        encoded_method: DexEncodedMethod {
            method_index: 0,
            access_flags: 0,
            code_offset: 1,
            kind: crate::dex::DexMethodKind::Virtual,
        },
        method: DexMethodId {
            class_descriptor: "Lcom/example/Slot;".to_owned(),
            name: "address".to_owned(),
            proto: crate::dex::DexProto {
                shorty: "II".to_owned(),
                return_type: "I".to_owned(),
                parameters: vec!["I".to_owned()],
            },
        },
    };
    let handler = DexRunInvocationHandler {
        current_dex_file_index: 0,
        current_class_descriptor: None,
        program: &program,
        depth: 0,
    };

    let size = handler
        .apply_method_intrinsic(
            &resolved_size,
            MethodIntrinsic::SlotGroupSize {
                receiver_arg: 0,
                group_array_field_index: 6,
                group_gap_size_field_index: 8,
            },
            &test_invocation(vec![receiver.clone()]),
        )
        .expect("size result");
    let before_gap = handler
        .apply_method_intrinsic(
            &resolved_address,
            MethodIntrinsic::SlotGroupAddress {
                receiver_arg: 0,
                index_arg: 1,
                group_gap_start_field_index: 7,
                group_gap_size_field_index: 8,
            },
            &test_invocation(vec![receiver.clone(), DexValue::Int(1)]),
        )
        .expect("before gap result");
    let after_gap = handler
        .apply_method_intrinsic(
            &resolved_address,
            MethodIntrinsic::SlotGroupAddress {
                receiver_arg: 0,
                index_arg: 1,
                group_gap_start_field_index: 7,
                group_gap_size_field_index: 8,
            },
            &test_invocation(vec![receiver, DexValue::Int(2)]),
        )
        .expect("after gap result");

    assert_eq!(size, Some(DexInvocationResult::Value(DexValue::Int(3))));
    assert_eq!(
        before_gap,
        Some(DexInvocationResult::Value(DexValue::Int(1)))
    );
    assert_eq!(
        after_gap,
        Some(DexInvocationResult::Value(DexValue::Int(3)))
    );
}

#[test]
fn dispatches_only_virtual_or_interface_descriptor_receivers() {
    let method = DexMethodId {
        class_descriptor: "Lk8/a;".to_owned(),
        name: "invoke".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let object_invocation = DexInvocation {
        kind: DexInvokeKind::Direct,
        range: false,
        method_index: 0,
        argument_registers: smallvec::smallvec![0],
        arguments: smallvec::smallvec![DexValue::Object(DexObjectRef {
            id: 7,
            class_descriptor: std::rc::Rc::from("Landroidx/compose/foundation/style/c;"),
        })],
    };
    let reference_invocation = DexInvocation {
        kind: DexInvokeKind::Interface,
        range: false,
        method_index: 0,
        argument_registers: smallvec::smallvec![0],
        arguments: smallvec::smallvec![DexValue::Reference(std::rc::Rc::from(
            "Landroidx/compose/foundation/style/c;.bootstrap",
        ))],
    };

    let _ = &method;
    assert_eq!(receiver_dispatch_descriptor(&object_invocation), None);

    let receiver =
        receiver_dispatch_descriptor(&reference_invocation).expect("method should dispatch");
    assert_eq!(receiver, "Landroidx/compose/foundation/style/c;");
}

#[test]
fn seeds_object_receiver_for_instance_entrypoints() {
    let program = DexProgram::new(Vec::new());
    let resolved = ResolvedMethod {
        dex_file_index: 0,
        dex_file_name: "classes.dex".to_owned(),
        class_descriptor: "Lcom/example/MainActivity;".to_owned(),
        encoded_method: DexEncodedMethod {
            method_index: 0,
            access_flags: 0,
            code_offset: 1,
            kind: crate::dex::DexMethodKind::Virtual,
        },
        method: DexMethodId {
            class_descriptor: "Lcom/example/MainActivity;".to_owned(),
            name: "onCreate".to_owned(),
            proto: crate::dex::DexProto {
                shorty: "VL".to_owned(),
                return_type: "V".to_owned(),
                parameters: vec!["Landroid/os/Bundle;".to_owned()],
            },
        },
    };
    let code = DexCodeItem {
        registers_size: 2,
        ins_size: 2,
        outs_size: 0,
        tries_size: 0,
        debug_info_offset: 0,
        instructions: Vec::new(),
        try_blocks: Vec::new(),
    };

    assert_eq!(
        program.entrypoint_initial_arguments(&resolved, &code),
        Some(vec![
            DexValue::Object(DexObjectRef {
                id: 1,
                class_descriptor: std::rc::Rc::from("Lcom/example/MainActivity;")
            }),
            DexValue::Null
        ])
    );
}

#[test]
fn seeds_static_entrypoint_parameters_with_default_slots() {
    let program = DexProgram::new(Vec::new());
    let resolved = ResolvedMethod {
        dex_file_index: 0,
        dex_file_name: "classes.dex".to_owned(),
        class_descriptor: "Lcom/example/MainActivity;".to_owned(),
        encoded_method: DexEncodedMethod {
            method_index: 0,
            access_flags: DEX_ACC_STATIC,
            code_offset: 1,
            kind: crate::dex::DexMethodKind::Direct,
        },
        method: DexMethodId {
            class_descriptor: "Lcom/example/MainActivity;".to_owned(),
            name: "main".to_owned(),
            proto: crate::dex::DexProto {
                shorty: "VIJL".to_owned(),
                return_type: "V".to_owned(),
                parameters: vec![
                    "I".to_owned(),
                    "J".to_owned(),
                    "[Ljava/lang/String;".to_owned(),
                ],
            },
        },
    };
    let code = DexCodeItem {
        registers_size: 4,
        ins_size: 4,
        outs_size: 0,
        tries_size: 0,
        debug_info_offset: 0,
        instructions: Vec::new(),
        try_blocks: Vec::new(),
    };

    assert_eq!(
        program.entrypoint_initial_arguments(&resolved, &code),
        Some(vec![
            DexValue::Int(0),
            DexValue::Long(0),
            DexValue::Long(0),
            DexValue::Null
        ])
    );
}

#[test]
fn matches_exact_runtime_types_for_instance_of() {
    let program = DexProgram::new(Vec::new());
    let activity = DexValue::Object(DexObjectRef {
        id: 1,
        class_descriptor: std::rc::Rc::from("Lcom/example/Activity;"),
    });

    assert!(program.value_matches_type(&activity, "Lcom/example/Activity;"));
    assert!(program.value_matches_type(&activity, "Ljava/lang/Object;"));
    assert!(!program.value_matches_type(&activity, "Lcom/example/Other;"));
    assert!(program.value_matches_type(
        &DexValue::Reference(std::rc::Rc::from("Landroid/os/Looper;:main")),
        "Landroid/os/Looper;"
    ));
    assert!(program.value_matches_type(
        &DexValue::String(std::rc::Rc::from("value")),
        "Ljava/lang/String;"
    ));
    assert!(!program.value_matches_type(&DexValue::Null, "Ljava/lang/Object;"));
}

#[test]
fn matches_apk_declared_interfaces_for_instance_of() {
    let dex_file = crate::dex::parse_dex_file("classes.dex", minimal_interface_dex())
        .expect("DEX should parse");
    let program = DexProgram::new(vec![dex_file]);
    let value = DexValue::Object(DexObjectRef {
        id: 1,
        class_descriptor: std::rc::Rc::from("Lcom/example/Test;"),
    });

    assert!(program.value_matches_type(&value, "Lcom/example/Marker;"));
}

// cite: libcore Class.java#Class
#[test]
fn libcore_class_matches_reflect_type_for_instance_of() {
    let program = DexProgram::new(Vec::new());
    let class = DexValue::Reference(std::rc::Rc::from("Ljava/lang/Class;:Ljava/util/ArrayList;"));

    assert!(program.value_matches_type(&class, "Ljava/lang/reflect/Type;"));
}

// cite: libcore Class.java#Class
// cite: libcore TypeDescriptor.java#OfField, TypeDescriptor
#[test]
fn libcore_class_matches_type_descriptor_for_instance_of() {
    let program = DexProgram::new(Vec::new());
    let class = DexValue::Reference(std::rc::Rc::from("Ljava/lang/Class;:Ljava/util/ArrayList;"));

    assert!(program.value_matches_type(&class, "Ljava/lang/invoke/TypeDescriptor;"));
}

// cite: libcore WeakReference.java#WeakReference
#[test]
fn libcore_weak_reference_matches_reference_for_instance_of() {
    let program = DexProgram::new(Vec::new());
    let weak_reference = DexValue::Object(DexObjectRef {
        id: 1,
        class_descriptor: std::rc::Rc::from("Ljava/lang/ref/WeakReference;"),
    });

    assert!(program.value_matches_type(&weak_reference, "Ljava/lang/ref/Reference;"));
}

#[test]
// cite: frameworks-base OnBackAnimationCallback.java#OnBackAnimationCallback
fn loaded_callback_matches_android_on_back_invoked_interface_transitively() {
    let dex_file = crate::dex::parse_dex_file(
        "classes.dex",
        minimal_external_interface_implementation_dex(
            "Lcom/example/Callback;",
            "Landroid/window/OnBackAnimationCallback;",
        ),
    )
    .expect("DEX should parse");
    let program = DexProgram::new(vec![dex_file]);
    let callback = DexValue::Object(DexObjectRef {
        id: 1,
        class_descriptor: std::rc::Rc::from("Lcom/example/Callback;"),
    });

    assert!(program.value_matches_type(&callback, "Landroid/window/OnBackInvokedCallback;"));
}

#[test]
fn matches_android_activity_context_hierarchy_for_instance_of() {
    let program = DexProgram::new(Vec::new());
    let activity = DexValue::Object(DexObjectRef {
        id: 1,
        class_descriptor: std::rc::Rc::from("Landroid/app/Activity;"),
    });

    assert!(program.value_matches_type(&activity, "Landroid/content/ContextWrapper;"));
    assert!(program.value_matches_type(&activity, "Landroid/content/Context;"));
}

#[test]
fn matches_android_bundle_basebundle_parcelable_hierarchy_for_instance_of() {
    let program = DexProgram::new(Vec::new());
    let bundle = DexValue::Object(DexObjectRef {
        id: 1,
        class_descriptor: std::rc::Rc::from("Landroid/os/Bundle;"),
    });

    assert!(program.value_matches_type(&bundle, "Landroid/os/BaseBundle;"));
    assert!(program.value_matches_type(&bundle, "Landroid/os/Parcelable;"));
    assert!(program.value_matches_type(&bundle, "Ljava/lang/Cloneable;"));
    assert!(program.value_matches_type(&bundle, "Ljava/lang/Object;"));
}

// cite: frameworks-base SQLiteCursor.java#SQLiteCursor
// cite: frameworks-base AbstractWindowedCursor.java#AbstractWindowedCursor
// cite: frameworks-base AbstractCursor.java#AbstractCursor
// cite: frameworks-base CrossProcessCursor.java#CrossProcessCursor
#[test]
fn frameworks_sqlite_cursor_matches_cursor_interface_hierarchy() {
    let program = DexProgram::new(Vec::new());
    let cursor = DexValue::Reference(std::rc::Rc::from(
        "Landroid/database/sqlite/SQLiteCursor;:sqlite:test",
    ));

    assert!(program.value_matches_type(&cursor, "Landroid/database/AbstractWindowedCursor;"));
    assert!(program.value_matches_type(&cursor, "Landroid/database/AbstractCursor;"));
    assert!(program.value_matches_type(&cursor, "Landroid/database/CrossProcessCursor;"));
    assert!(program.value_matches_type(&cursor, "Landroid/database/Cursor;"));
}

// cite: frameworks-base BoringLayout.java#BoringLayout
// cite: frameworks-base Layout.java#Layout
#[test]
fn frameworks_boring_layout_matches_layout_superclass() {
    let program = DexProgram::new(Vec::new());
    let layout = DexValue::Object(DexObjectRef {
        id: 1,
        class_descriptor: std::rc::Rc::from("Landroid/text/BoringLayout;"),
    });

    assert!(program.value_matches_type(&layout, "Landroid/text/Layout;"));
    assert!(program.value_matches_type(&layout, "Ljava/lang/Object;"));
}

#[test]
fn matches_java_cancellation_exception_throwable_hierarchy_for_instance_of() {
    let program = DexProgram::new(Vec::new());
    let cancellation = DexValue::Object(DexObjectRef {
        id: 1,
        class_descriptor: std::rc::Rc::from("Ljava/util/concurrent/CancellationException;"),
    });

    assert!(program.value_matches_type(&cancellation, "Ljava/lang/IllegalStateException;"));
    assert!(program.value_matches_type(&cancellation, "Ljava/lang/Throwable;"));
}

// cite: libcore NullPointerException.java#NullPointerException
#[test]
fn libcore_null_pointer_exception_matches_runtime_exception_and_throwable() {
    let program = DexProgram::new(Vec::new());

    assert!(program.descriptor_matches_type(
        "Ljava/lang/NullPointerException;",
        "Ljava/lang/RuntimeException;"
    ));
    assert!(program
        .descriptor_matches_type("Ljava/lang/NullPointerException;", "Ljava/lang/Throwable;"));
}

// cite: art interpreter_switch_impl-inl.h#THROW, MOVE_EXCEPTION
// cite: art interpreter_common.cc#MoveToExceptionHandler
// cite: art art_method.cc#FindCatchBlock
#[test]
fn art_throw_null_allocates_guest_exception_and_matches_superclass_handler() {
    let program = DexProgram::new(Vec::new());
    let mut handler = DexRunInvocationHandler {
        program: &program,
        current_dex_file_index: 0,
        current_class_descriptor: None,
        depth: 0,
    };
    let code = DexCodeItem {
        registers_size: 1,
        ins_size: 0,
        outs_size: 0,
        tries_size: 1,
        debug_info_offset: 0,
        instructions: vec![0x0012, 0x0027, 0x000d, 0x0011],
        try_blocks: vec![crate::dex::DexTryBlock {
            start_address: 1,
            instruction_count: 1,
            handlers: vec![crate::dex::DexExceptionHandler {
                type_descriptor: Some("Ljava/lang/RuntimeException;".to_owned()),
                address: 2,
            }],
        }],
    };

    let result = crate::dex_interpreter::interpret_code_with_initial_arguments_in_context(
        &code,
        None,
        &mut handler,
        0,
    )
    .expect("throw null should execute through the production exception path");
    let DexExecutionResult::Value(exception) = result else {
        panic!("typed handler should return its generated exception: {result}");
    };

    assert!(program.value_matches_type(&exception, "Ljava/lang/NullPointerException;"));
    assert!(program.value_matches_type(&exception, "Ljava/lang/RuntimeException;"));
    assert!(program.value_matches_type(&exception, "Ljava/lang/Throwable;"));
}

// cite: art thread.cc#ThrowNewException, ThrowNewWrappedException
// cite: libcore Throwable.java#Throwable, detailMessage
#[test]
fn art_generated_exception_allocation_stores_detail_message() {
    let program = DexProgram::new(Vec::new());
    let exception = program.allocate_generated_exception(DexGeneratedException {
        class_descriptor: "Ljava/lang/NullPointerException;".to_owned(),
        detail_message: Some("message".to_owned()),
    });
    let detail_message = DexFieldId {
        class_descriptor: "Ljava/lang/Throwable;".to_owned(),
        type_descriptor: "Ljava/lang/String;".to_owned(),
        name: "detailMessage".to_owned(),
    };

    assert_eq!(
        program.read_stored_instance_field(&exception, &detail_message),
        Some(DexValue::String(std::rc::Rc::from("message")))
    );
}

// cite: art interpreter_switch_impl-inl.h#CHECK_CAST
// cite: art class-inl.h#Class::IsAssignableFrom
#[test]
fn art_check_cast_accepts_subtype_null_and_encoded_null() {
    let dex_file =
        crate::dex::parse_dex_file("classes.dex", minimal_inherited_shadow_field_class_dex())
            .expect("DEX should parse");
    let program = DexProgram::new(vec![dex_file]);
    let code = DexCodeItem {
        registers_size: 1,
        ins_size: 1,
        outs_size: 0,
        tries_size: 0,
        debug_info_offset: 0,
        instructions: vec![0x001f, 0x0000, 0x0011],
        try_blocks: Vec::new(),
    };
    let child = program.allocate_object("Lcom/example/Child;".to_owned());
    let mut child_handler = DexRunInvocationHandler {
        program: &program,
        current_dex_file_index: 0,
        current_class_descriptor: None,
        depth: 0,
    };
    let mut null_handler = DexRunInvocationHandler {
        program: &program,
        current_dex_file_index: 0,
        current_class_descriptor: None,
        depth: 0,
    };
    let mut encoded_null_handler = DexRunInvocationHandler {
        program: &program,
        current_dex_file_index: 0,
        current_class_descriptor: None,
        depth: 0,
    };

    assert_eq!(
        crate::dex_interpreter::interpret_code_with_initial_arguments_in_context(
            &code,
            Some(std::slice::from_ref(&child)),
            &mut child_handler,
            0,
        )
        .expect("subtype check-cast should execute"),
        DexExecutionResult::Value(child)
    );
    assert_eq!(
        crate::dex_interpreter::interpret_code_with_initial_arguments_in_context(
            &code,
            Some(&[DexValue::Null]),
            &mut null_handler,
            0,
        )
        .expect("null check-cast should execute"),
        DexExecutionResult::Value(DexValue::Null)
    );
    assert_eq!(
        crate::dex_interpreter::interpret_code_with_initial_arguments_in_context(
            &code,
            Some(&[DexValue::Int(0)]),
            &mut encoded_null_handler,
            0,
        )
        .expect("encoded null check-cast should execute"),
        DexExecutionResult::Value(DexValue::Int(0))
    );
}

// cite: art method_verifier.cc#CodeFlowVerifyInstruction
#[test]
fn art_verifier_rejects_check_cast_primitive_targets_and_non_reference_values() {
    let dex_file =
        crate::dex::parse_dex_file("classes.dex", minimal_inherited_shadow_field_class_dex())
            .expect("DEX should parse");
    let program = DexProgram::new(vec![dex_file]);
    let primitive_target = DexCodeItem {
        registers_size: 1,
        ins_size: 1,
        outs_size: 0,
        tries_size: 0,
        debug_info_offset: 0,
        instructions: vec![0x001f, 0x0002, 0x0011],
        try_blocks: Vec::new(),
    };
    let reference_target = DexCodeItem {
        instructions: vec![0x001f, 0x0000, 0x0011],
        ..primitive_target.clone()
    };
    let mut primitive_target_handler = DexRunInvocationHandler {
        program: &program,
        current_dex_file_index: 0,
        current_class_descriptor: None,
        depth: 0,
    };
    let mut non_reference_handler = DexRunInvocationHandler {
        program: &program,
        current_dex_file_index: 0,
        current_class_descriptor: None,
        depth: 0,
    };

    let primitive_target_error =
        crate::dex_interpreter::interpret_code_with_initial_arguments_in_context(
            &primitive_target,
            Some(&[DexValue::Null]),
            &mut primitive_target_handler,
            0,
        )
        .expect_err("primitive check-cast target should fail before the null shortcut");
    let non_reference_error =
        crate::dex_interpreter::interpret_code_with_initial_arguments_in_context(
            &reference_target,
            Some(&[DexValue::Int(1)]),
            &mut non_reference_handler,
            0,
        )
        .expect_err("non-reference check-cast value should fail explicitly");

    assert!(primitive_target_error
        .to_string()
        .contains("check-cast target is not a reference type"));
    assert!(non_reference_error
        .to_string()
        .contains("check-cast expected a reference in v0"));
}

// cite: art interpreter_switch_impl-inl.h#CHECK_CAST
// cite: art class-inl.h#Class::IsAssignableFromArray, Class::IsArrayAssignableFromArray
// cite: art common_throws.cc#ThrowClassCastException
// cite: art descriptors_names.cc#AppendPrettyDescriptor, PrettyDescriptor
#[test]
fn art_check_cast_uses_array_component_assignability_and_pretty_failure_names() {
    let dex_file =
        crate::dex::parse_dex_file("classes.dex", minimal_list_builder_field_class_dex())
            .expect("DEX should parse");
    let program = DexProgram::new(vec![dex_file]);
    let code = DexCodeItem {
        registers_size: 1,
        ins_size: 1,
        outs_size: 0,
        tries_size: 0,
        debug_info_offset: 0,
        instructions: vec![0x001f, 0x0004, 0x0011],
        try_blocks: Vec::new(),
    };
    let strings = program
        .allocate_array("[Ljava/lang/String;".to_owned(), 0)
        .expect("reference array should allocate");
    let nested_primitives = program
        .allocate_array("[[I".to_owned(), 0)
        .expect("nested primitive array should allocate");
    let reference_strings = DexValue::Reference(std::rc::Rc::from("[Ljava/lang/String;:params"));
    let primitives = DexValue::Reference(std::rc::Rc::from("[I:styleable"));

    for compatible in [strings, nested_primitives, reference_strings] {
        let mut handler = DexRunInvocationHandler {
            program: &program,
            current_dex_file_index: 0,
            current_class_descriptor: None,
            depth: 0,
        };

        assert_eq!(
            crate::dex_interpreter::interpret_code_with_initial_arguments_in_context(
                &code,
                Some(std::slice::from_ref(&compatible)),
                &mut handler,
                0,
            )
            .expect("array check-cast should execute"),
            DexExecutionResult::Value(compatible)
        );
    }

    let mut handler = DexRunInvocationHandler {
        program: &program,
        current_dex_file_index: 0,
        current_class_descriptor: None,
        depth: 0,
    };
    let result = crate::dex_interpreter::interpret_code_with_initial_arguments_in_context(
        &code,
        Some(&[primitives]),
        &mut handler,
        0,
    )
    .expect("incompatible array check-cast should remain a guest exception");
    let DexExecutionResult::Throw(exception) = result else {
        panic!("incompatible array check-cast should throw: {result}");
    };
    let detail_message = DexFieldId {
        class_descriptor: "Ljava/lang/Throwable;".to_owned(),
        type_descriptor: "Ljava/lang/String;".to_owned(),
        name: "detailMessage".to_owned(),
    };

    assert_eq!(
        program.read_stored_instance_field(&exception, &detail_message),
        Some(DexValue::String(std::rc::Rc::from(
            "int[] cannot be cast to java.lang.Object[]"
        )))
    );
}

// cite: art interpreter_switch_impl-inl.h#CHECK_CAST
// cite: art common_throws.cc#ThrowClassCastException
// cite: art interpreter_common.cc#MoveToExceptionHandler
// cite: libcore ClassCastException.java#ClassCastException
#[test]
fn art_failed_check_cast_is_catchable_by_runtime_exception_handler() {
    let dex_file =
        crate::dex::parse_dex_file("classes.dex", minimal_inherited_shadow_field_class_dex())
            .expect("DEX should parse");
    let program = DexProgram::new(vec![dex_file]);
    let code = DexCodeItem {
        registers_size: 1,
        ins_size: 1,
        outs_size: 0,
        tries_size: 1,
        debug_info_offset: 0,
        instructions: vec![0x001f, 0x0001, 0x0011, 0x000d, 0x0011],
        try_blocks: vec![crate::dex::DexTryBlock {
            start_address: 0,
            instruction_count: 2,
            handlers: vec![crate::dex::DexExceptionHandler {
                type_descriptor: Some("Ljava/lang/RuntimeException;".to_owned()),
                address: 3,
            }],
        }],
    };
    let parent = program.allocate_object("Lcom/example/Parent;".to_owned());
    let mut handler = DexRunInvocationHandler {
        program: &program,
        current_dex_file_index: 0,
        current_class_descriptor: None,
        depth: 0,
    };

    let result = crate::dex_interpreter::interpret_code_with_initial_arguments_in_context(
        &code,
        Some(&[parent]),
        &mut handler,
        0,
    )
    .expect("failed check-cast should remain a guest exception");
    let DexExecutionResult::Value(exception) = result else {
        panic!("runtime exception handler should return the cast failure: {result}");
    };
    let detail_message = DexFieldId {
        class_descriptor: "Ljava/lang/Throwable;".to_owned(),
        type_descriptor: "Ljava/lang/String;".to_owned(),
        name: "detailMessage".to_owned(),
    };

    assert!(program.value_matches_type(&exception, "Ljava/lang/ClassCastException;"));
    assert!(program.value_matches_type(&exception, "Ljava/lang/RuntimeException;"));
    assert!(program.value_matches_type(&exception, "Ljava/lang/Throwable;"));
    assert_eq!(
        program.read_stored_instance_field(&exception, &detail_message),
        Some(DexValue::String(std::rc::Rc::from(
            "com.example.Parent cannot be cast to com.example.Child"
        )))
    );
}

// cite: art interpreter_switch_impl-inl.h#CHECK_CAST
// cite: art common_throws.cc#ThrowClassCastException
#[test]
fn art_uncaught_check_cast_failure_returns_guest_exception() {
    let dex_file =
        crate::dex::parse_dex_file("classes.dex", minimal_inherited_shadow_field_class_dex())
            .expect("DEX should parse");
    let program = DexProgram::new(vec![dex_file]);
    let code = DexCodeItem {
        registers_size: 1,
        ins_size: 1,
        outs_size: 0,
        tries_size: 0,
        debug_info_offset: 0,
        instructions: vec![0x001f, 0x0001, 0x0011],
        try_blocks: Vec::new(),
    };
    let parent = program.allocate_object("Lcom/example/Parent;".to_owned());
    let mut handler = DexRunInvocationHandler {
        program: &program,
        current_dex_file_index: 0,
        current_class_descriptor: None,
        depth: 0,
    };

    let result = crate::dex_interpreter::interpret_code_with_initial_arguments_in_context(
        &code,
        Some(&[parent]),
        &mut handler,
        0,
    )
    .expect("failed check-cast should not become a host error");
    let DexExecutionResult::Throw(exception) = result else {
        panic!("uncaught check-cast should throw a guest exception: {result}");
    };

    assert!(program.value_matches_type(&exception, "Ljava/lang/ClassCastException;"));
}

fn run_new_instance_of_type(
    class_descriptor: &str,
    other_descriptor: &str,
    type_index: u16,
) -> (DexProgram, DexExecutionResult) {
    let dex_file = crate::dex::parse_dex_file(
        "classes.dex",
        minimal_external_interface_implementation_dex(class_descriptor, other_descriptor),
    )
    .expect("DEX should parse");
    let program = DexProgram::new(vec![dex_file]);
    let code = DexCodeItem {
        registers_size: 1,
        ins_size: 0,
        outs_size: 0,
        tries_size: 0,
        debug_info_offset: 0,
        instructions: vec![0x0022, type_index, 0x0011],
        try_blocks: Vec::new(),
    };
    let result = {
        let mut handler = DexRunInvocationHandler {
            program: &program,
            current_dex_file_index: 0,
            current_class_descriptor: None,
            depth: 0,
        };
        crate::dex_interpreter::interpret_code_with_initial_arguments_in_context(
            &code,
            None,
            &mut handler,
            0,
        )
        .expect("new-instance should not become a host error")
    };

    (program, result)
}

// cite: art class_linker.cc#DoResolveType, ThrowNoClassDefFoundError
// cite: art interpreter_switch_impl-inl.h#NEW_INSTANCE
#[test]
fn art_new_instance_of_nested_class_without_class_def_throws_no_class_def_found_error() {
    let (program, result) =
        run_new_instance_of_type("Lcom/example/Outer;", "Lcom/example/Outer$2;", 1);
    let DexExecutionResult::Throw(exception) = result else {
        panic!("new-instance of a missing nested class should throw: {result}");
    };
    let detail_message = DexFieldId {
        class_descriptor: "Ljava/lang/Throwable;".to_owned(),
        type_descriptor: "Ljava/lang/String;".to_owned(),
        name: "detailMessage".to_owned(),
    };

    assert!(program.value_matches_type(&exception, "Ljava/lang/NoClassDefFoundError;"));
    assert!(program.value_matches_type(&exception, "Ljava/lang/LinkageError;"));
    assert!(program.value_matches_type(&exception, "Ljava/lang/Error;"));
    assert!(!program.value_matches_type(&exception, "Ljava/lang/Exception;"));
    assert_eq!(
        program.read_stored_instance_field(&exception, &detail_message),
        Some(DexValue::String(std::rc::Rc::from(
            "Failed resolution of: Lcom/example/Outer$2;"
        )))
    );
}

// cite: art class_linker.cc#DoResolveType
#[test]
fn art_new_instance_allocates_defined_and_framework_classes() {
    let (_, defined) = run_new_instance_of_type("Lcom/example/Outer;", "Lcom/example/Outer$2;", 0);
    let (_, framework) = run_new_instance_of_type("Lcom/example/Outer;", "Landroid/os/Handler;", 1);
    let (_, unknown_outer) =
        run_new_instance_of_type("Lcom/example/Outer;", "Lcom/example/Other$1;", 1);

    assert!(matches!(defined, DexExecutionResult::Value(DexValue::Object(_))));
    assert!(matches!(framework, DexExecutionResult::Value(DexValue::Object(_))));
    assert!(matches!(unknown_outer, DexExecutionResult::Value(DexValue::Object(_))));
}

// cite: libcore ClassLoader.java#loadClass
// cite: art class_linker.cc#ClassLinker::FindClassInBaseDexClassLoader
#[test]
fn art_app_class_loader_ignores_duplicate_boot_class_in_apk_dex() {
    let descriptor = "Landroid/app/Activity;";
    let dex_file = crate::dex::parse_dex_file("classes.dex", minimal_class_stub_dex(descriptor))
        .expect("DEX should parse");
    let program = DexProgram::new(vec![dex_file]);
    let activity = DexValue::Object(DexObjectRef {
        id: 1,
        class_descriptor: std::rc::Rc::from(descriptor),
    });

    assert!(program.load_class(descriptor).is_none());
    assert_eq!(
        program.class_get_class_loader(&test_invocation(vec![DexValue::Reference(
            std::rc::Rc::from(format!("Ljava/lang/Class;:{descriptor}")),
        )])),
        Some(DexValue::Reference(std::rc::Rc::from(
            "Ljava/lang/ClassLoader;.bootstrap"
        )))
    );
    assert!(program.value_matches_type(&activity, "Landroid/content/ContextWrapper;"));
}

// cite: libcore ClassLoader.java#loadClass
// cite: art class_linker.cc#ClassLinker::FindClassInBaseDexClassLoader
#[test]
fn art_app_class_loader_loads_apk_class_absent_from_boot_class_path() {
    let descriptor = "Lcom/example/ApkOwned;";
    let dex_file = crate::dex::parse_dex_file("classes.dex", minimal_class_stub_dex(descriptor))
        .expect("DEX should parse");
    let program = DexProgram::new(vec![dex_file]);

    let loaded = program
        .load_class(descriptor)
        .expect("APK class should load");
    assert_eq!(loaded.class_def.descriptor, descriptor);
    assert_eq!(
        program.class_get_class_loader(&test_invocation(vec![DexValue::Reference(
            std::rc::Rc::from(format!("Ljava/lang/Class;:{descriptor}")),
        )])),
        Some(DexValue::Reference(std::rc::Rc::from(
            "Ljava/lang/ClassLoader;.app"
        )))
    );
}

#[test]
fn stores_and_reads_allocated_object_instance_fields() {
    let program = DexProgram::new(Vec::new());
    let object = program.allocate_object("Lexample/Object;".to_owned());
    let field = DexFieldId {
        class_descriptor: "Lexample/Object;".to_owned(),
        type_descriptor: "I".to_owned(),
        name: "value".to_owned(),
    };

    assert!(program.write_stored_instance_field(&object, &field, DexValue::Int(7)));
    assert_eq!(
        program.read_stored_instance_field(&object, &field),
        Some(DexValue::Int(7))
    );
}

#[test]
fn stores_filled_array_values_by_allocated_array_identity() {
    let program = DexProgram::new(Vec::new());
    let array = program.allocate_filled_array(
        "[Ljava/lang/String;".to_owned(),
        vec![
            DexValue::String(std::rc::Rc::from("left")),
            DexValue::String(std::rc::Rc::from("right")),
        ],
    );
    let array_key = map_identity_key(&array);

    assert_eq!(
        array,
        DexValue::Object(DexObjectRef {
            id: 1,
            class_descriptor: std::rc::Rc::from("[Ljava/lang/String;")
        })
    );
    assert_eq!(
        program.runtime.borrow().array_entries.get(&array_key),
        Some(&vec![
            DexValue::String(std::rc::Rc::from("left")),
            DexValue::String(std::rc::Rc::from("right"))
        ])
    );
    assert_eq!(
        program.read_array_value(&array, 1),
        Some(DexValue::String(std::rc::Rc::from("right")))
    );
}

#[test]
fn stores_new_array_length_by_allocated_array_identity() {
    let program = DexProgram::new(Vec::new());
    let array = program
        .allocate_array("[Ljava/lang/Object;".to_owned(), 3)
        .expect("array should allocate");
    let array_key = map_identity_key(&array);

    assert_eq!(
        array,
        DexValue::Object(DexObjectRef {
            id: 1,
            class_descriptor: std::rc::Rc::from("[Ljava/lang/Object;")
        })
    );
    assert_eq!(
        program.runtime.borrow().array_entries.get(&array_key),
        Some(&vec![DexValue::Null, DexValue::Null, DexValue::Null])
    );
    assert_eq!(program.array_length(&array), Some(3));
}

#[test]
fn stores_new_primitive_array_with_zero_values() {
    let program = DexProgram::new(Vec::new());
    let array = program
        .allocate_array("[I".to_owned(), 2)
        .expect("array should allocate");
    let float_array = program
        .allocate_array("[F".to_owned(), 2)
        .expect("float array should allocate");
    let array_key = map_identity_key(&array);
    let float_array_key = map_identity_key(&float_array);

    assert_eq!(
        program.runtime.borrow().array_entries.get(&array_key),
        Some(&vec![DexValue::Int(0), DexValue::Int(0)])
    );
    assert_eq!(
        program.runtime.borrow().array_entries.get(&float_array_key),
        Some(&vec![DexValue::Int(0), DexValue::Int(0)])
    );
    assert_eq!(program.array_length(&array), Some(2));
    assert_eq!(program.array_length(&float_array), Some(2));
}

#[test]
fn stores_array_writes_by_allocated_array_identity() {
    let program = DexProgram::new(Vec::new());
    let array = program
        .allocate_array("[Ljava/lang/Object;".to_owned(), 2)
        .expect("array should allocate");
    let value = DexValue::String(std::rc::Rc::from("stored"));

    assert_eq!(
        program.write_array_value(&array, 1, value.clone()),
        Some(())
    );
    assert_eq!(program.read_array_value(&array, 1), Some(value));
}

#[test]
fn stores_float_array_writes_as_raw_bits() {
    let program = DexProgram::new(Vec::new());
    let array = program
        .allocate_array("[F".to_owned(), 2)
        .expect("array should allocate");

    assert_eq!(
        program.write_array_value(&array, 1, DexValue::Float(1.5)),
        Some(())
    );
    assert_eq!(
        program.read_array_value(&array, 1),
        Some(DexValue::Int(1.5_f32.to_bits() as i32))
    );
}

#[test]
fn builtin_array_clone_copies_values_to_distinct_array() {
    let program = DexProgram::new(Vec::new());
    let array_clone = DexMethodId {
        class_descriptor: "[Ljava/lang/Object;".to_owned(),
        name: "clone".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let object_clone = DexMethodId {
        class_descriptor: "Ljava/lang/Object;".to_owned(),
        name: "clone".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let value = DexValue::String(std::rc::Rc::from("stored"));
    let array =
        program.allocate_filled_array("[Ljava/lang/Object;".to_owned(), vec![value.clone()]);
    let invocation = test_invocation(vec![array.clone()]);

    let Some(DexExecutionResult::Value(cloned)) =
        program.invoke_builtin_method(&array_clone, Some(&invocation))
    else {
        panic!("array clone did not return a cloned array");
    };
    let Some(DexExecutionResult::Value(object_cloned)) =
        program.invoke_builtin_method(&object_clone, Some(&invocation))
    else {
        panic!("Object.clone did not return a cloned array");
    };

    assert_ne!(cloned, array);
    assert_ne!(object_cloned, array);
    assert_eq!(program.array_length(&cloned), Some(1));
    assert_eq!(program.array_length(&object_cloned), Some(1));
    assert_eq!(program.read_array_value(&cloned, 0), Some(value));
}

// cite: art object.cc#Clone
#[test]
fn art_array_clone_preserves_receiver_runtime_class() {
    let program = DexProgram::new(Vec::new());
    let array_clone = DexMethodId {
        class_descriptor: "[Ljava/lang/Object;".to_owned(),
        name: "clone".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let array = program.allocate_filled_array(
        "[Lexample/Kind;".to_owned(),
        vec![DexValue::Object(DexObjectRef {
            id: 1,
            class_descriptor: std::rc::Rc::from("Lexample/Kind;"),
        })],
    );

    let Some(DexExecutionResult::Value(cloned)) =
        program.invoke_builtin_method(&array_clone, Some(&test_invocation(vec![array.clone()])))
    else {
        panic!("array clone did not return a cloned array");
    };

    assert_ne!(cloned, array);
    assert_eq!(value_class_descriptor(&cloned), Some("[Lexample/Kind;"));
    assert_eq!(
        program.read_array_value(&cloned, 0),
        program.read_array_value(&array, 0)
    );
}

#[test]
fn stores_and_reads_reference_backed_instance_fields() {
    let program = DexProgram::new(Vec::new());
    let object = DexValue::Reference(std::rc::Rc::from("Lexample/Object;.bootstrap"));
    let field = DexFieldId {
        class_descriptor: "Lexample/Object;".to_owned(),
        type_descriptor: "Z".to_owned(),
        name: "enabled".to_owned(),
    };

    assert!(program.write_stored_instance_field(&object, &field, DexValue::Int(1)));
    assert_eq!(
        program.read_stored_instance_field(&object, &field),
        Some(DexValue::Int(1))
    );
}

#[test]
fn art_object_inl_reference_fields_stay_out_of_allocated_object_store() {
    let program = DexProgram::new(Vec::new());
    let object = DexValue::Reference(std::rc::Rc::from("Lexample/Object;.bootstrap"));
    let owner_key = instance_field_owner_key(&object).expect("reference should have an owner key");
    let field = DexFieldId {
        class_descriptor: "Lexample/Object;".to_owned(),
        type_descriptor: "Z".to_owned(),
        name: "enabled".to_owned(),
    };
    let signature = field.display_signature();

    assert!(program.write_stored_instance_field(&object, &field, DexValue::Int(1)));

    let runtime = program.runtime.borrow();
    assert_eq!(
        runtime
            .reference_fields
            .reference_field("Lexample/Object;.bootstrap", &signature),
        Some(&DexValue::Int(1))
    );
    assert_eq!(
        runtime.field(&owner_key, &signature),
        Some(&DexValue::Int(1))
    );
    assert_eq!(
        runtime.instance_fields.get_field(&owner_key, &signature),
        None
    );
    assert!(runtime
        .instance_fields
        .iter_objects()
        .all(|(owner, _)| !owner.matches_key(&owner_key)));
}

#[test]
fn art_class_linker_linkfields_resolves_instance_field_layout_metadata() {
    let dex_file =
        crate::dex::parse_dex_file("classes.dex", minimal_list_builder_field_class_dex())
            .expect("DEX should parse");
    let program = DexProgram::new(vec![dex_file]);

    let array = program
        .resolved_instance_field(0, 0)
        .expect("field should resolve")
        .expect("field should have app class metadata");
    let size = program
        .resolved_instance_field(0, 1)
        .expect("field should resolve")
        .expect("field should have app class metadata");

    assert_eq!(array.declaring_class, "Lcom/example/ListBuilder;");
    assert_eq!(array.type_descriptor, "[Ljava/lang/Object;");
    assert_eq!(array.access_flags, JAVA_ACC_PUBLIC);
    assert!(!array.is_volatile);
    assert_eq!(array.slot, 0);
    assert_eq!(array.default_value, Some(DexValue::Null));
    assert_eq!(size.type_descriptor, "I");
    assert_eq!(size.slot, 1);
    assert_eq!(size.default_value, Some(DexValue::Int(0)));
}

#[test]
fn art_object_inl_resolved_instance_slots_read_and_write_allocated_objects() {
    let dex_file =
        crate::dex::parse_dex_file("classes.dex", minimal_list_builder_field_class_dex())
            .expect("DEX should parse");
    let program = DexProgram::new(vec![dex_file]);
    let object = program.allocate_object("Lcom/example/ListBuilder;".to_owned());
    let resolved = program
        .resolved_instance_field(0, 1)
        .expect("field should resolve")
        .expect("field should have app class metadata");

    assert_eq!(
        program
            .read_stored_instance_field_resolved(&resolved, &object)
            .expect("read should not fail"),
        None
    );
    assert!(program
        .write_stored_instance_field_resolved(&resolved, &object, DexValue::Int(42))
        .expect("write should not fail"));
    assert_eq!(
        program
            .read_stored_instance_field_resolved(&resolved, &object)
            .expect("read should not fail"),
        Some(DexValue::Int(42))
    );
}

#[test]
fn slot_backed_fields_iter_skips_unset_leading_and_middle_slots() {
    let mut store = InstanceFieldStore::default();
    let signatures = vec![
        std::rc::Rc::from("Lexample/Sparse;.first:I"),
        std::rc::Rc::from("Lexample/Sparse;.second:I"),
        std::rc::Rc::from("Lexample/Sparse;.third:I"),
        std::rc::Rc::from("Lexample/Sparse;.fourth:I"),
    ];
    store.define_class_layout("Lexample/Sparse;", &signatures);
    store.register_object_id(1, "Lexample/Sparse;".to_owned());
    assert!(store.set_object_field_slot(1, 1, DexValue::Int(20)));
    assert!(store.set_object_field_slot(1, 3, DexValue::Int(40)));

    let fields = store
        .fields_of_object(1)
        .expect("registered object should have fields");
    let mut fields = fields.iter();
    let (second_signature, second_value) = fields.next().expect("second slot should be visible");
    let (fourth_signature, fourth_value) = fields.next().expect("fourth slot should be visible");

    assert_eq!(second_signature, signatures[1].as_ref());
    assert_eq!(second_value, &DexValue::Int(20));
    assert_eq!(fourth_signature, signatures[3].as_ref());
    assert_eq!(fourth_value, &DexValue::Int(40));
    assert!(fields.next().is_none());
}

// cite: art class_linker.cc#LinkFields
#[test]
fn art_inherited_field_iteration_skips_unset_shadow_slot_without_hiding_later_field() {
    let dex_file =
        crate::dex::parse_dex_file("classes.dex", minimal_inherited_shadow_field_class_dex())
            .expect("DEX should parse");
    let program = DexProgram::new(vec![dex_file]);
    let object = program.allocate_object("Lcom/example/Child;".to_owned());
    let parent_value = program
        .resolved_instance_field(0, 0)
        .expect("field should resolve")
        .expect("parent field should have metadata");
    let child_value = program
        .resolved_instance_field(0, 1)
        .expect("field should resolve")
        .expect("child shadow field should have metadata");
    let child_only = program
        .resolved_instance_field(0, 2)
        .expect("field should resolve")
        .expect("later child field should have metadata");

    assert_eq!(parent_value.slot, 0);
    assert_eq!(child_value.slot, 1);
    assert_eq!(child_only.slot, 2);
    assert!(program
        .write_stored_instance_field_resolved(&parent_value, &object, DexValue::Int(10))
        .expect("parent write should not fail"));
    assert!(program
        .write_stored_instance_field_resolved(&child_only, &object, DexValue::Int(30))
        .expect("later child write should not fail"));

    let DexValue::Object(object) = object else {
        panic!("allocated child should be an object");
    };
    let runtime = program.runtime.borrow();
    let fields = runtime
        .instance_fields
        .fields_of_object(object.id)
        .expect("allocated child should have fields");
    let mut fields = fields.iter();
    let (parent_signature, parent_stored_value) =
        fields.next().expect("parent field should be visible");
    let (child_signature, child_stored_value) = fields
        .next()
        .expect("later child field should remain visible");

    assert_eq!(parent_signature, parent_value.signature.as_ref());
    assert_eq!(parent_stored_value, &DexValue::Int(10));
    assert_eq!(child_signature, child_only.signature.as_ref());
    assert_eq!(child_stored_value, &DexValue::Int(30));
    assert!(fields.next().is_none());
}

#[test]
fn art_class_linker_linkfields_keeps_inherited_and_shadowed_instance_slots_distinct() {
    let dex_file =
        crate::dex::parse_dex_file("classes.dex", minimal_inherited_shadow_field_class_dex())
            .expect("DEX should parse");
    let program = DexProgram::new(vec![dex_file]);
    let object = program.allocate_object("Lcom/example/Child;".to_owned());
    let parent_value = program
        .resolved_instance_field(0, 0)
        .expect("field should resolve")
        .expect("parent field should have metadata");
    let child_value = program
        .resolved_instance_field(0, 1)
        .expect("field should resolve")
        .expect("child field should have metadata");
    let child_only = program
        .resolved_instance_field(0, 2)
        .expect("field should resolve")
        .expect("child field should have metadata");

    assert_eq!(parent_value.declaring_class, "Lcom/example/Parent;");
    assert_eq!(child_value.declaring_class, "Lcom/example/Child;");
    assert_eq!(parent_value.slot, 0);
    assert_eq!(child_value.slot, 1);
    assert_eq!(child_only.slot, 2);

    assert!(program
        .write_stored_instance_field_resolved(&parent_value, &object, DexValue::Int(10))
        .expect("parent write should not fail"));
    assert!(program
        .write_stored_instance_field_resolved(&child_value, &object, DexValue::Int(20))
        .expect("child write should not fail"));
    assert!(program
        .write_stored_instance_field_resolved(&child_only, &object, DexValue::Int(30))
        .expect("child-only write should not fail"));

    assert_eq!(
        program
            .read_stored_instance_field_resolved(&parent_value, &object)
            .expect("parent read should not fail"),
        Some(DexValue::Int(10))
    );
    assert_eq!(
        program
            .read_stored_instance_field_resolved(&child_value, &object)
            .expect("child read should not fail"),
        Some(DexValue::Int(20))
    );
    assert_eq!(
        program
            .read_stored_instance_field_resolved(&child_only, &object)
            .expect("child-only read should not fail"),
        Some(DexValue::Int(30))
    );
}

#[test]
fn stores_and_reads_static_fields() {
    let program = DexProgram::new(Vec::new());
    let field = DexFieldId {
        class_descriptor: "Lexample/Singleton;".to_owned(),
        type_descriptor: "Ljava/lang/Object;".to_owned(),
        name: "instance".to_owned(),
    };

    program.write_stored_static_field(&field, DexValue::Null);
    assert_eq!(
        program.read_stored_static_field(&field),
        Some(DexValue::Null)
    );
}

#[test]
fn art_class_linker_linkstaticfields_resolves_static_slots_by_type_order() {
    let dex_file = crate::dex::parse_dex_file("classes.dex", minimal_static_fields_class_dex())
        .expect("DEX should parse");
    let program = DexProgram::new(vec![dex_file]);
    let count = program
        .resolved_static_field(0, 0)
        .expect("field should resolve")
        .expect("static field should have metadata");
    let shared = program
        .resolved_static_field(0, 1)
        .expect("field should resolve")
        .expect("static field should have metadata");
    let total = program
        .resolved_static_field(0, 2)
        .expect("field should resolve")
        .expect("static field should have metadata");
    let enabled = program
        .resolved_static_field(0, 3)
        .expect("field should resolve")
        .expect("static field should have metadata");

    assert_eq!(shared.slot, 0);
    assert_eq!(total.slot, 1);
    assert_eq!(count.slot, 2);
    assert_eq!(enabled.slot, 3);
    assert_eq!(count.default_value, Some(DexValue::Int(0)));
    assert_eq!(shared.default_value, Some(DexValue::Null));
}

#[test]
fn art_class_linker_initializeclass_seeds_static_values_by_dex_field_order() {
    let dex_file = crate::dex::parse_dex_file("classes.dex", minimal_static_fields_class_dex())
        .expect("DEX should parse");
    let program = DexProgram::new(vec![dex_file]);
    let count = program
        .resolved_static_field(0, 0)
        .expect("field should resolve")
        .expect("static field should have metadata");
    let shared = program
        .resolved_static_field(0, 1)
        .expect("field should resolve")
        .expect("static field should have metadata");
    let total = program
        .resolved_static_field(0, 2)
        .expect("field should resolve")
        .expect("static field should have metadata");
    let enabled = program
        .resolved_static_field(0, 3)
        .expect("field should resolve")
        .expect("static field should have metadata");

    program
        .ensure_class_initialized("Lcom/example/Statics;", 0)
        .expect("class initialization should succeed");

    assert_eq!(
        program.read_stored_static_field_resolved(&count),
        Some(DexValue::Int(7))
    );
    assert_eq!(
        program.read_stored_static_field_resolved(&shared),
        Some(DexValue::String(std::rc::Rc::from("seed")))
    );
    assert_eq!(
        program.read_stored_static_field_resolved(&total),
        Some(DexValue::Long(11))
    );
    assert_eq!(
        program.read_stored_static_field_resolved(&enabled),
        Some(DexValue::Int(1))
    );

    assert!(program.write_stored_static_field_resolved(&count, DexValue::Int(42)));
    assert_eq!(
        program.read_stored_static_field_resolved(&count),
        Some(DexValue::Int(42))
    );
}

#[test]
fn defaults_unstored_object_and_int_fields() {
    let object_field = DexFieldId {
        class_descriptor: "Lexample/Singleton;".to_owned(),
        type_descriptor: "Ljava/lang/Object;".to_owned(),
        name: "instance".to_owned(),
    };
    let int_field = DexFieldId {
        class_descriptor: "Lexample/Flags;".to_owned(),
        type_descriptor: "I".to_owned(),
        name: "value".to_owned(),
    };

    assert_eq!(default_field_value(&object_field), Some(DexValue::Null));
    assert_eq!(default_field_value(&int_field), Some(DexValue::Int(0)));
}

#[test]
fn defaults_only_unstored_primitive_instance_fields() {
    let object_field = DexFieldId {
        class_descriptor: "Lexample/Object;".to_owned(),
        type_descriptor: "Ljava/lang/Object;".to_owned(),
        name: "dependency".to_owned(),
    };
    let boolean_field = DexFieldId {
        class_descriptor: "Lexample/Object;".to_owned(),
        type_descriptor: "Z".to_owned(),
        name: "enabled".to_owned(),
    };
    let float_field = DexFieldId {
        class_descriptor: "Lexample/Object;".to_owned(),
        type_descriptor: "F".to_owned(),
        name: "scale".to_owned(),
    };
    let long_field = DexFieldId {
        class_descriptor: "Lexample/Object;".to_owned(),
        type_descriptor: "J".to_owned(),
        name: "timestamp".to_owned(),
    };

    assert_eq!(default_primitive_field_value(&object_field), None);
    assert_eq!(
        default_primitive_field_value(&boolean_field),
        Some(DexValue::Int(0))
    );
    assert_eq!(
        default_primitive_field_value(&float_field),
        Some(DexValue::Float(0.0))
    );
    assert_eq!(
        default_primitive_field_value(&long_field),
        Some(DexValue::Long(0))
    );
}
