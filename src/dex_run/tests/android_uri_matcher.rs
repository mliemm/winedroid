use super::super::*;
use super::helpers::*;

fn matcher_method(name: &str, return_type: &str, parameters: &[&str]) -> DexMethodId {
    builtin_method(
        "Landroid/content/UriMatcher;",
        name,
        return_type,
        parameters,
    )
}

fn matcher_call(
    program: &DexProgram,
    method: &DexMethodId,
    arguments: Vec<DexValue>,
) -> DexExecutionResult {
    program
        .invoke_uri_matcher_method(method, &test_invocation(arguments))
        .expect("UriMatcher call should not fail")
        .expect("UriMatcher method should be handled")
}

fn text(value: &str) -> DexValue {
    DexValue::String(std::rc::Rc::from(value))
}

fn new_matcher(program: &DexProgram, code: i32) -> DexValue {
    let matcher = program.allocate_object("Landroid/content/UriMatcher;".to_owned());
    assert_eq!(
        matcher_call(
            program,
            &matcher_method("<init>", "V", &["I"]),
            vec![matcher.clone(), DexValue::Int(code)]
        ),
        DexExecutionResult::Void
    );
    matcher
}

fn add_uri_method() -> DexMethodId {
    matcher_method(
        "addURI",
        "V",
        &["Ljava/lang/String;", "Ljava/lang/String;", "I"],
    )
}

fn add(program: &DexProgram, matcher: &DexValue, authority: DexValue, path: DexValue, code: i32) {
    let result = matcher_call(
        program,
        &add_uri_method(),
        vec![matcher.clone(), authority, path, DexValue::Int(code)],
    );
    assert_eq!(result, DexExecutionResult::Void);
}

fn uri(program: &DexProgram, value: &str) -> DexValue {
    program
        .parse_uri(&test_invocation(vec![text(value)]))
        .expect("Uri.parse should build a Uri")
}

fn matched(program: &DexProgram, matcher: &DexValue, uri: DexValue) -> DexExecutionResult {
    matcher_call(
        program,
        &matcher_method("match", "I", &["Landroid/net/Uri;"]),
        vec![matcher.clone(), uri],
    )
}

#[test]
fn cts_uri_matcher_match_returns_the_added_codes_and_no_match_for_unknown() {
    // cite: cts UriMatcherTest.java#testMatch, testAddURI
    let program = DexProgram::new(Vec::new());
    let matcher = new_matcher(&program, -1);
    let paths = ["testPath1", "testPath2", "testPath3", "testPath4"];
    for path in paths {
        assert_eq!(
            matched(
                &program,
                &matcher,
                uri(&program, &format!("content://testAuthority/{path}"))
            ),
            DexExecutionResult::Int(-1)
        );
    }
    for (index, path) in paths.iter().enumerate() {
        add(
            &program,
            &matcher,
            text("testAuthority"),
            text(path),
            index as i32 + 1,
        );
    }

    for (index, path) in paths.iter().enumerate() {
        assert_eq!(
            matched(
                &program,
                &matcher,
                uri(&program, &format!("content://testAuthority/{path}"))
            ),
            DexExecutionResult::Int(index as i32 + 1)
        );
    }
    assert_eq!(
        matched(&program, &matcher, uri(&program, "abc")),
        DexExecutionResult::Int(-1)
    );
}

#[test]
fn cts_uri_matcher_rejects_null_uri_and_negative_code() {
    // cite: cts UriMatcherTest.java#testMatchFailure, testAddURIFailure
    let program = DexProgram::new(Vec::new());
    let matcher = new_matcher(&program, -1);

    let DexExecutionResult::Throw(exception) = matched(&program, &matcher, DexValue::Null) else {
        panic!("match(null) must throw");
    };
    assert!(program.value_matches_type(&exception, "Ljava/lang/NullPointerException;"));

    let DexExecutionResult::Throw(exception) = matcher_call(
        &program,
        &add_uri_method(),
        vec![matcher, DexValue::Null, DexValue::Null, DexValue::Int(-1)],
    ) else {
        panic!("addURI with a negative code must throw");
    };
    assert!(program.value_matches_type(&exception, "Ljava/lang/IllegalArgumentException;"));
}

#[test]
fn frameworks_uri_matcher_number_and_text_wildcards_and_leading_slash() {
    // cite: frameworks-base UriMatcher.java#addURI, match, createChild
    let program = DexProgram::new(Vec::new());
    let matcher = new_matcher(&program, 99);
    add(&program, &matcher, text("contacts"), text("people"), 1);
    add(&program, &matcher, text("contacts"), text("people/#"), 2);
    add(
        &program,
        &matcher,
        text("contacts"),
        text("phones/filter/*"),
        3,
    );
    add(&program, &matcher, text("contacts"), text("/slash"), 4);
    add(&program, &matcher, text("bare"), DexValue::Null, 5);

    let expectations = [
        ("content://contacts/people", 1),
        ("content://contacts/people/12", 2),
        ("content://contacts/people/x", -1),
        ("content://contacts/phones/filter/anything", 3),
        ("content://contacts/phones/filter", -1),
        ("content://contacts/slash", 4),
        ("content://bare", 5),
        ("content://contacts", -1),
        ("content://other/people", -1),
    ];
    for (value, code) in expectations {
        assert_eq!(
            matched(&program, &matcher, uri(&program, value)),
            DexExecutionResult::Int(code),
            "{value}"
        );
    }
}

#[test]
fn frameworks_uri_matcher_root_code_matches_an_empty_uri() {
    // cite: frameworks-base UriMatcher.java#match
    let program = DexProgram::new(Vec::new());
    let matcher = new_matcher(&program, 99);

    assert_eq!(
        matched(&program, &matcher, uri(&program, "")),
        DexExecutionResult::Int(99)
    );
}
