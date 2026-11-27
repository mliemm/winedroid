use super::super::*;

// cite: libcore Writer.java#write
#[test]
fn writer_write_string_dispatches_to_the_character_array_override() {
    let dex_file = crate::dex::parse_dex_file(
        "writer_contract.dex",
        include_bytes!("fixtures/writer_contract.dex").to_vec(),
    )
    .expect("writer contract DEX should parse");
    let program = DexProgram::new(vec![dex_file]);
    let method = DexMethodId {
        class_descriptor: "Lwinedroid/tests/WriterContract;".to_owned(),
        name: "writeString".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let resolved = program
        .resolve_exact_method(&method)
        .expect("writer contract method should resolve")
        .expect("writer contract method should exist");

    assert_eq!(
        program
            .execute_method(None, &resolved, None, 0)
            .expect("writer contract method should execute"),
        DexExecutionResult::Int(1)
    );
}
