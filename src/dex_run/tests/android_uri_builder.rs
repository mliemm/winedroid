use super::super::*;
use super::helpers::*;

fn text(value: &str) -> DexValue {
    DexValue::String(std::rc::Rc::from(value))
}

fn uri(program: &DexProgram, value: &str) -> DexValue {
    program
        .parse_uri(&test_invocation(vec![text(value)]))
        .expect("Uri.parse should build a Uri")
}

fn call(
    program: &DexProgram,
    class_descriptor: &str,
    name: &str,
    return_type: &str,
    parameters: &[&str],
    arguments: Vec<DexValue>,
) -> Option<DexValue> {
    let method = builtin_method(class_descriptor, name, return_type, parameters);
    match program
        .invoke_uri_builder_method(&method, &test_invocation(arguments))
        .expect("Uri builder call should not fail")?
    {
        DexExecutionResult::Value(value) => Some(value),
        other => panic!("unexpected result {other:?}"),
    }
}

fn build_upon(program: &DexProgram, uri: DexValue) -> Option<DexValue> {
    call(
        program,
        "Landroid/net/Uri;",
        "buildUpon",
        "Landroid/net/Uri$Builder;",
        &[],
        vec![uri],
    )
}

fn append_path(program: &DexProgram, builder: &DexValue, segment: DexValue) -> DexValue {
    call(
        program,
        "Landroid/net/Uri$Builder;",
        "appendPath",
        "Landroid/net/Uri$Builder;",
        &["Ljava/lang/String;"],
        vec![builder.clone(), segment],
    )
    .expect("appendPath should return the builder")
}

fn built_text(program: &DexProgram, builder: &DexValue) -> String {
    let built = call(
        program,
        "Landroid/net/Uri$Builder;",
        "build",
        "Landroid/net/Uri;",
        &[],
        vec![builder.clone()],
    )
    .expect("build should return a Uri");
    program
        .uri_text(&built)
        .expect("built Uri carries its text")
}

#[test]
fn cts_content_uris_append_id_builds_the_expected_uri_string() {
    // cite: cts ContentUrisTest.java#testAppendId
    let program = DexProgram::new(Vec::new());
    for (base, expected) in [
        (
            "content://testAuthority/testPath1",
            "content://testAuthority/testPath1/12",
        ),
        (
            "content://testAuthority/testPath2/",
            "content://testAuthority/testPath2/12",
        ),
    ] {
        let builder = build_upon(&program, uri(&program, base)).expect("hierarchical buildUpon");
        let builder = append_path(&program, &builder, text("12"));

        assert_eq!(built_text(&program, &builder), expected);
    }
}

#[test]
fn frameworks_uri_builder_append_path_encodes_the_segment_and_keeps_query_and_fragment() {
    // cite: frameworks-base Uri.java#encode, appendDecodedSegment, buildUpon, build
    let program = DexProgram::new(Vec::new());
    let builder = build_upon(&program, uri(&program, "content://auth?a=1#frag")).unwrap();
    let builder = append_path(&program, &builder, text("a b/c\u{e9}"));

    assert_eq!(
        built_text(&program, &builder),
        "content://auth/a%20b%2Fc%C3%A9?a=1#frag"
    );
}

#[test]
fn frameworks_uri_builder_prepends_slash_when_authority_present_and_path_relative() {
    // cite: frameworks-base Uri.java#makeAbsolute, build, parsePath
    let program = DexProgram::new(Vec::new());
    let backslash = build_upon(&program, uri(&program, "http://host\\leaf")).unwrap();

    assert_eq!(built_text(&program, &backslash), "http://host/\\leaf");

    let builder = build_upon(&program, uri(&program, "file:/base")).unwrap();
    let builder = append_path(&program, &builder, text("leaf"));

    assert_eq!(built_text(&program, &builder), "file:/base/leaf");

    let relative = build_upon(&program, uri(&program, "rel/path")).unwrap();
    assert_eq!(built_text(&program, &relative), "rel/path");
}

#[test]
fn frameworks_uri_build_upon_on_an_opaque_uri_is_a_missing_primitive() {
    // cite: frameworks-base Uri.java#isHierarchical, buildUpon
    let program = DexProgram::new(Vec::new());

    assert!(build_upon(&program, uri(&program, "mailto:a@b.c")).is_none());
}
