use super::super::*;

fn generic_signature_contract_program() -> DexProgram {
    let dex_file = crate::dex::parse_dex_file(
        "generic_signature_contract.dex",
        include_bytes!("fixtures/generic_signature_contract.dex").to_vec(),
    )
    .expect("generic signature contract DEX should parse");
    DexProgram::new(vec![dex_file])
}

fn execute_generic_signature_contract_method(name: &str) -> i32 {
    let program = generic_signature_contract_program();
    let method = DexMethodId {
        class_descriptor: "Lwinedroid/tests/GenericSignatureContract;".to_owned(),
        name: name.to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let resolved = program
        .resolve_exact_method(&method)
        .expect("generic signature contract method should resolve")
        .expect("generic signature contract method should exist");
    let result = program
        .execute_method(None, &resolved, None, 0)
        .expect("generic signature contract method should execute");

    match result {
        DexExecutionResult::Int(value) => value,
        other => panic!("generic signature contract method `{name}` did not return int: {other:?}"),
    }
}

// cite: libcore GenericSignatureParser.java#parseClassSignature
// cite: libcore Class.java#getGenericSuperclass
#[test]
fn generic_superclass_exposes_parameterized_raw_type_and_arguments() {
    assert_eq!(
        execute_generic_signature_contract_method("directSuperclass"),
        1
    );
}

// cite: libcore Arrays.java#hashCode, equals
#[test]
fn arrays_hash_and_equals_use_structural_type_equality() {
    assert_eq!(
        execute_generic_signature_contract_method("arraysHashCode"),
        1
    );
}

// cite: libcore Class.java#getEnclosingClass, getDeclaringClass
#[test]
fn enclosing_class_annotations_resolve() {
    assert_eq!(
        execute_generic_signature_contract_method("enclosingClasses"),
        1
    );
}

// cite: libcore GenericSignatureParser.java#parseTypeArgument
// cite: libcore ParameterizedTypeImpl.java#getActualTypeArguments, getRawType
#[test]
fn generic_type_arguments_nest() {
    assert_eq!(
        execute_generic_signature_contract_method("nestedTypeArguments"),
        1
    );
}

// cite: libcore GenericSignatureParser.java#parseTypeArgument
// cite: libcore WildcardTypeImpl.java#getUpperBounds, getLowerBounds
#[test]
fn generic_wildcard_bounds_resolve() {
    assert_eq!(
        execute_generic_signature_contract_method("typeTokenShape"),
        1
    );
}

// cite: libcore TypeVariableImpl.java#getName, getBounds, getGenericDeclaration
// cite: libcore Class.java#getTypeParameters
#[test]
fn class_type_parameters_resolve() {
    assert_eq!(
        execute_generic_signature_contract_method("typeParameters"),
        1
    );
}

// cite: libcore Class.java#getGenericSuperclass
#[test]
fn generic_superclass_without_type_arguments_stays_a_class() {
    assert_eq!(
        execute_generic_signature_contract_method("missingTypeParameter"),
        1
    );
}
