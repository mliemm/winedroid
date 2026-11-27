use super::super::*;

// cite: libcore Collection.java#removeIf
#[test]
fn array_list_remove_if_dispatches_predicate_and_compacts() {
    let dex_file = crate::dex::parse_dex_file(
        "remove_if_contract.dex",
        include_bytes!("fixtures/remove_if_contract.dex").to_vec(),
    )
    .expect("removeIf contract DEX should parse");
    let program = DexProgram::new(vec![dex_file]);
    let method = DexMethodId {
        class_descriptor: "Lwinedroid/tests/RemoveIfContract;".to_owned(),
        name: "removeOdd".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let resolved = program
        .resolve_exact_method(&method)
        .expect("removeIf contract method should resolve")
        .expect("removeIf contract method should exist");

    assert_eq!(
        program
            .execute_method(None, &resolved, None, 0)
            .expect("removeIf contract method should execute"),
        DexExecutionResult::Int(1)
    );
}
