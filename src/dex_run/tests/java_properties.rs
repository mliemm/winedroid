use super::super::*;
use super::helpers::*;

fn method(
    class_descriptor: &str,
    name: &str,
    return_type: &str,
    parameters: &[&str],
) -> DexMethodId {
    DexMethodId {
        class_descriptor: class_descriptor.to_owned(),
        name: name.to_owned(),
        proto: crate::dex::DexProto {
            shorty: String::new(),
            return_type: return_type.to_owned(),
            parameters: parameters.iter().map(|value| (*value).to_owned()).collect(),
        },
    }
}

fn text(value: &str) -> DexValue {
    DexValue::String(std::rc::Rc::from(value))
}

fn new_properties(program: &DexProgram) -> DexValue {
    let properties = program.allocate_object("Ljava/util/Properties;".to_owned());
    let init = method("Ljava/util/Properties;", "<init>", "V", &[]);
    assert_eq!(
        program.invoke_builtin_method(&init, Some(&test_invocation(vec![properties.clone()]))),
        Some(DexExecutionResult::Void)
    );

    properties
}

fn load(program: &DexProgram, properties: &DexValue, bytes: &[u8]) -> Option<DexExecutionResult> {
    let stream = program.allocate_object("Ljava/io/FileInputStream;".to_owned());
    program.runtime.borrow_mut().input_stream_values.insert(
        map_identity_key(&stream),
        InputStreamState {
            bytes: bytes.to_vec(),
            position: 0,
            pending_low_surrogate: None,
        },
    );
    let load = method(
        "Ljava/util/Properties;",
        "load",
        "V",
        &["Ljava/io/InputStream;"],
    );

    program.invoke_builtin_method(
        &load,
        Some(&test_invocation(vec![properties.clone(), stream])),
    )
}

fn entries(program: &DexProgram, properties: &DexValue) -> Vec<(String, String)> {
    let map_key = map_identity_key(properties);
    let runtime = program.runtime.borrow();
    let mut entries: Vec<(String, String)> = runtime
        .map_entries
        .iter()
        .filter(|((owner, _), _)| *owner == map_key)
        .map(|(key, value)| {
            let (DexValue::String(name), DexValue::String(value)) =
                (&runtime.map_entry_keys[key], value)
            else {
                panic!("property entries are strings");
            };
            (name.to_string(), value.to_string())
        })
        .collect();
    entries.sort();

    entries
}

fn pair(key: &str, value: &str) -> (String, String) {
    (key.to_owned(), value.to_owned())
}

fn thrown_class(result: Option<DexExecutionResult>) -> String {
    let Some(DexExecutionResult::Throw(exception)) = result else {
        panic!("expected a thrown exception, got {result:?}");
    };

    value_class_descriptor(&exception).unwrap().to_owned()
}

#[test]
fn libcore_properties_constructor_makes_empty_hashtable_subclass() {
    // cite: libcore Properties.java#Properties
    // cite: libcore Hashtable.java#Hashtable
    let program = DexProgram::new(Vec::new());
    let properties = new_properties(&program);

    assert!(entries(&program, &properties).is_empty());
    for supertype in [
        "Ljava/util/Hashtable;",
        "Ljava/util/Dictionary;",
        "Ljava/util/Map;",
        "Ljava/io/Serializable;",
    ] {
        assert!(
            program.value_matches_type(&properties, supertype),
            "{supertype}"
        );
    }
}

#[test]
fn libcore_properties_load_parses_separators_comments_continuations_and_escapes() {
    // cite: libcore Properties.java#load, load0, LineReader, loadConvert
    let cases: [(&[u8], Vec<(String, String)>); 15] = [
        (b"a=b", vec![pair("a", "b")]),
        (b"a b", vec![pair("a", "b")]),
        (b"a:b", vec![pair("a", "b")]),
        (b" a = b", vec![pair("a", "b")]),
        (b"#x\n!y\nk=v", vec![pair("k", "v")]),
        (b"k=a\\\n   b", vec![pair("k", "ab")]),
        (b"k=a\\\r\n   b", vec![pair("k", "ab")]),
        (b"\\u0041=\\n", vec![pair("A", "\n")]),
        (b"a\\=b=c", vec![pair("a=b", "c")]),
        (b"foo", vec![pair("foo", "")]),
        (b"=v", vec![pair("", "v")]),
        (b"a=1\r\nb=2\r\n\r\n", vec![pair("a", "1"), pair("b", "2")]),
        (b"a=1\na=2", vec![pair("a", "2")]),
        (b"a=\xc3\xa9", vec![pair("a", "\u{c3}\u{a9}")]),
        (b"a=\xe9", vec![pair("a", "\u{e9}")]),
    ];

    for (input, expected) in cases {
        let program = DexProgram::new(Vec::new());
        let properties = new_properties(&program);

        assert_eq!(
            load(&program, &properties, input),
            Some(DexExecutionResult::Void),
            "{input:?}"
        );
        assert_eq!(entries(&program, &properties), expected, "{input:?}");
    }
}

#[test]
fn libcore_properties_load_malformed_unicode_escape_throws_after_earlier_entries() {
    // cite: libcore Properties.java#load0, loadConvert
    let program = DexProgram::new(Vec::new());
    let properties = new_properties(&program);

    let result = load(&program, &properties, b"ok=1\nbad=\\u00zz\n");

    assert_eq!(thrown_class(result), "Ljava/lang/IllegalArgumentException;");
    assert_eq!(entries(&program, &properties), vec![pair("ok", "1")]);
}

#[test]
fn libcore_properties_load_reads_to_end_of_stream() {
    // cite: libcore Properties.java#load, LineReader
    let program = DexProgram::new(Vec::new());
    let properties = new_properties(&program);
    let stream = program.allocate_object("Ljava/io/FileInputStream;".to_owned());
    program.runtime.borrow_mut().input_stream_values.insert(
        map_identity_key(&stream),
        InputStreamState {
            bytes: b"a=b".to_vec(),
            position: 0,
            pending_low_surrogate: None,
        },
    );
    let load = method(
        "Ljava/util/Properties;",
        "load",
        "V",
        &["Ljava/io/InputStream;"],
    );

    program.invoke_builtin_method(
        &load,
        Some(&test_invocation(vec![properties, stream.clone()])),
    );

    assert_eq!(
        program.runtime.borrow().input_stream_values[&map_identity_key(&stream)].position,
        3
    );
}

#[test]
fn libcore_properties_load_null_stream_throws_and_unknown_stream_is_missing_primitive() {
    // cite: libcore Properties.java#load, LineReader
    let program = DexProgram::new(Vec::new());
    let properties = new_properties(&program);
    let load = method(
        "Ljava/util/Properties;",
        "load",
        "V",
        &["Ljava/io/InputStream;"],
    );

    let null_stream = program.invoke_builtin_method(
        &load,
        Some(&test_invocation(vec![properties.clone(), DexValue::Null])),
    );
    let unknown_stream = program.invoke_builtin_method(
        &load,
        Some(&test_invocation(vec![
            properties,
            program.allocate_object("Lcom/example/Stream;".to_owned()),
        ])),
    );

    assert_eq!(
        thrown_class(null_stream),
        "Ljava/lang/NullPointerException;"
    );
    assert_eq!(unknown_stream, None);
}

#[test]
fn libcore_hashtable_contains_key_hits_misses_and_rejects_null_key() {
    // cite: libcore Hashtable.java#containsKey
    let program = DexProgram::new(Vec::new());
    let properties = new_properties(&program);
    load(&program, &properties, b"present=1");
    let contains_key = method(
        "Ljava/util/Properties;",
        "containsKey",
        "Z",
        &["Ljava/lang/Object;"],
    );
    let ask = |receiver: &DexValue, key: DexValue| {
        program
            .invoke_java_collection_equality_method(
                &contains_key,
                &test_invocation(vec![receiver.clone(), key]),
                0,
            )
            .expect("Hashtable.containsKey should run")
    };

    assert_eq!(
        ask(&properties, text("present")),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        ask(&properties, text("absent")),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        thrown_class(ask(&properties, DexValue::Null)),
        "Ljava/lang/NullPointerException;"
    );
}

#[test]
fn libcore_hash_map_contains_key_accepts_null_key_unlike_hashtable() {
    // cite: libcore HashMap.java#containsKey
    let program = DexProgram::new(Vec::new());
    let map = program.allocate_object("Ljava/util/HashMap;".to_owned());
    let contains_key = method(
        "Ljava/util/Map;",
        "containsKey",
        "Z",
        &["Ljava/lang/Object;"],
    );

    assert_eq!(
        program
            .invoke_java_collection_equality_method(
                &contains_key,
                &test_invocation(vec![map, DexValue::Null]),
                0,
            )
            .expect("HashMap.containsKey should run"),
        Some(DexExecutionResult::Int(0))
    );
}
