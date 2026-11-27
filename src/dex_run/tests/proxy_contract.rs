use super::super::*;

fn proxy_contract_program() -> DexProgram {
    let dex_file = crate::dex::parse_dex_file(
        "proxy_contract.dex",
        include_bytes!("fixtures/proxy_contract.dex").to_vec(),
    )
    .expect("Proxy contract DEX should parse");
    DexProgram::new(vec![dex_file])
}

fn execute_proxy_contract_method(name: &str) -> i32 {
    let program = proxy_contract_program();
    let method = DexMethodId {
        class_descriptor: "Lwinedroid/tests/ProxyContract;".to_owned(),
        name: name.to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let resolved = program
        .resolve_exact_method(&method)
        .expect("Proxy contract method should resolve")
        .expect("Proxy contract method should exist");
    let result = program
        .execute_method(None, &resolved, None, 0)
        .expect("Proxy contract method should execute");

    match result {
        DexExecutionResult::Int(value) => value,
        other => panic!("Proxy contract method `{name}` did not return int: {other:?}"),
    }
}

// cite: libcore Proxy.java#newProxyInstance, invoke
#[test]
fn proxy_interface_dispatch_boxes_arguments_and_unboxes_results() {
    assert_eq!(execute_proxy_contract_method("interfaceDispatch"), 358);
}

// cite: libcore Proxy.java#equals, hashCode, toString
#[test]
fn proxy_object_methods_dispatch_to_invocation_handler() {
    assert_eq!(execute_proxy_contract_method("objectDispatch"), 7);
}

// cite: libcore Proxy.java#isProxyClass, getInvocationHandler
// cite: libcore Class.java#getSuperclass, getInterfaces
#[test]
fn proxy_class_reports_interfaces_and_proxy_superclass() {
    assert_eq!(execute_proxy_contract_method("typeIdentity"), 31);
}

// cite: art entrypoint_utils.cc#InvokeProxyInvocationHandler
#[test]
fn proxy_handler_exception_propagates_to_caller() {
    assert_eq!(execute_proxy_contract_method("handlerException"), 1);
}

// cite: libcore Proxy.java#ProxyClassFactory
#[test]
fn proxy_rejects_repeated_interfaces() {
    assert_eq!(execute_proxy_contract_method("repeatedInterface"), 1);
}

// cite: libcore Proxy.java#ProxyClassFactory
#[test]
fn proxy_rejects_non_interface_types() {
    assert_eq!(execute_proxy_contract_method("notAnInterface"), 1);
}

// cite: libcore Proxy.java#newProxyInstance
#[test]
fn proxy_rejects_null_invocation_handler() {
    assert_eq!(execute_proxy_contract_method("nullHandler"), 1);
}

#[test]
fn proxy_is_proxy_class_rejects_null_class() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/reflect/Proxy;".to_owned(),
        name: "isProxyClass".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/Class;".to_owned()],
        },
    };

    let Some(DexExecutionResult::Throw(exception)) = program.invoke_builtin_method(
        &method,
        Some(&super::helpers::test_invocation(vec![DexValue::Null])),
    ) else {
        panic!("isProxyClass should throw for a null class");
    };

    assert_eq!(
        value_class_descriptor(&exception),
        Some("Ljava/lang/NullPointerException;")
    );
}

#[test]
fn proxy_get_invocation_handler_rejects_non_proxy() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/reflect/Proxy;".to_owned(),
        name: "getInvocationHandler".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/lang/reflect/InvocationHandler;".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let invocation = super::helpers::test_invocation(vec![DexValue::Reference(std::rc::Rc::from(
        "Ljava/lang/Object;:not-a-proxy",
    ))]);
    let Some(DexExecutionResult::Throw(exception)) =
        program.invoke_builtin_method(&method, Some(&invocation))
    else {
        panic!("getInvocationHandler should throw for a non-proxy object");
    };

    assert_eq!(
        value_class_descriptor(&exception),
        Some("Ljava/lang/IllegalArgumentException;")
    );
}
