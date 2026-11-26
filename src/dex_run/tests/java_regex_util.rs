use super::super::*;
use super::helpers::*;

#[test]
fn builtin_regex_pattern_matcher_tracks_find_groups_and_utf16_indices() {
    let program = DexProgram::new(Vec::new());
    let compile = DexMethodId {
        class_descriptor: "Ljava/util/regex/Pattern;".to_owned(),
        name: "compile".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/util/regex/Pattern;".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned()],
        },
    };
    let matcher_method = DexMethodId {
        class_descriptor: "Ljava/util/regex/Pattern;".to_owned(),
        name: "matcher".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/util/regex/Matcher;".to_owned(),
            parameters: vec!["Ljava/lang/CharSequence;".to_owned()],
        },
    };
    let find = DexMethodId {
        class_descriptor: "Ljava/util/regex/Matcher;".to_owned(),
        name: "find".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let find_from = DexMethodId {
        class_descriptor: "Ljava/util/regex/Matcher;".to_owned(),
        name: "find".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZI".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let matcher_pattern = DexMethodId {
        class_descriptor: "Ljava/util/regex/Matcher;".to_owned(),
        name: "pattern".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/util/regex/Pattern;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let group = DexMethodId {
        class_descriptor: "Ljava/util/regex/Matcher;".to_owned(),
        name: "group".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let start = DexMethodId {
        class_descriptor: "Ljava/util/regex/Matcher;".to_owned(),
        name: "start".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let end = DexMethodId {
        class_descriptor: "Ljava/util/regex/Matcher;".to_owned(),
        name: "end".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };

    let Some(DexExecutionResult::Value(pattern)) = program.invoke_builtin_method(
        &compile,
        Some(&test_invocation(vec![DexValue::String(std::rc::Rc::from(
            r"\d+".to_owned(),
        ))])),
    ) else {
        panic!("Pattern.compile should return a Pattern object");
    };
    assert_eq!(
        value_class_descriptor(&pattern),
        Some("Ljava/util/regex/Pattern;")
    );

    let Some(DexExecutionResult::Value(matcher)) = program.invoke_builtin_method(
        &matcher_method,
        Some(&test_invocation(vec![
            pattern.clone(),
            DexValue::String(std::rc::Rc::from("a\u{1f600}12+34")),
        ])),
    ) else {
        panic!("Pattern.matcher should return a Matcher object");
    };
    assert_eq!(
        value_class_descriptor(&matcher),
        Some("Ljava/util/regex/Matcher;")
    );

    let Some(DexExecutionResult::Value(returned_pattern)) = program.invoke_builtin_method(
        &matcher_pattern,
        Some(&test_invocation(vec![matcher.clone()])),
    ) else {
        panic!("Matcher.pattern should return a Pattern object");
    };
    assert_eq!(
        value_class_descriptor(&returned_pattern),
        Some("Ljava/util/regex/Pattern;")
    );
    let Some(DexExecutionResult::Value(returned_pattern_matcher)) = program.invoke_builtin_method(
        &matcher_method,
        Some(&test_invocation(vec![
            returned_pattern,
            DexValue::String(std::rc::Rc::from("56")),
        ])),
    ) else {
        panic!("returned Pattern should create a Matcher object");
    };
    assert_eq!(
        program.invoke_builtin_method(
            &find,
            Some(&test_invocation(vec![returned_pattern_matcher]))
        ),
        Some(DexExecutionResult::Int(1))
    );

    assert_eq!(
        program.invoke_builtin_method(&find, Some(&test_invocation(vec![matcher.clone()]))),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&group, Some(&test_invocation(vec![matcher.clone()]))),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("12")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&start, Some(&test_invocation(vec![matcher.clone()]))),
        Some(DexExecutionResult::Int(3))
    );
    assert_eq!(
        program.invoke_builtin_method(&end, Some(&test_invocation(vec![matcher.clone()]))),
        Some(DexExecutionResult::Int(5))
    );
    assert_eq!(
        program.invoke_builtin_method(&find, Some(&test_invocation(vec![matcher.clone()]))),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&group, Some(&test_invocation(vec![matcher]))),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("34")
        )))
    );

    let Some(DexExecutionResult::Value(matcher_from)) = program.invoke_builtin_method(
        &matcher_method,
        Some(&test_invocation(vec![
            pattern,
            DexValue::String(std::rc::Rc::from("a\u{1f600}12+34")),
        ])),
    ) else {
        panic!("Pattern.matcher should return a Matcher object");
    };
    assert_eq!(
        program.invoke_builtin_method(
            &find_from,
            Some(&test_invocation(vec![
                matcher_from.clone(),
                DexValue::Int(6)
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&group, Some(&test_invocation(vec![matcher_from.clone()]))),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("34")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&start, Some(&test_invocation(vec![matcher_from.clone()]))),
        Some(DexExecutionResult::Int(6))
    );
    assert_eq!(
        program.invoke_builtin_method(&end, Some(&test_invocation(vec![matcher_from]))),
        Some(DexExecutionResult::Int(8))
    );
}

#[test]
fn builtin_calendar_get_instance_returns_bootstrap_calendar() {
    let program = DexProgram::new(Vec::new());
    let get_instance = DexMethodId {
        class_descriptor: "Ljava/util/Calendar;".to_owned(),
        name: "getInstance".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/util/Calendar;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get = DexMethodId {
        class_descriptor: "Ljava/util/Calendar;".to_owned(),
        name: "get".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "II".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let hour_invocation = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from("Ljava/util/Calendar;.bootstrap")),
        DexValue::Int(ANDROID_CALENDAR_HOUR_OF_DAY),
    ]);
    let unsupported_invocation = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from("Ljava/util/Calendar;.bootstrap")),
        DexValue::Int(1),
    ]);

    assert_eq!(
        program.invoke_builtin_method(&get_instance, None),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Ljava/util/Calendar;.bootstrap")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&get, Some(&hour_invocation)),
        Some(DexExecutionResult::Int(BOOTSTRAP_HOUR_OF_DAY))
    );
    assert_eq!(
        program.invoke_builtin_method(&get, Some(&unsupported_invocation)),
        None
    );
}

#[test]
fn builtin_kotlin_platform_random_returns_bootstrap_random() {
    let program = DexProgram::new(Vec::new());
    for class_descriptor in ["Lo8/a;", "LB1/a;"] {
        let method = DexMethodId {
            class_descriptor: class_descriptor.to_owned(),
            name: "a".to_owned(),
            proto: crate::dex::DexProto {
                shorty: "L".to_owned(),
                return_type: "Ljava/util/Random;".to_owned(),
                parameters: Vec::new(),
            },
        };

        assert_eq!(
            program.invoke_builtin_method(&method, None),
            Some(DexExecutionResult::Value(DexValue::Reference(
                std::rc::Rc::from("Ljava/util/Random;.bootstrap")
            )))
        );
    }
}

// cite: libcore Random.java#Random, nextInt, next
#[test]
fn libcore_random_next_int_bound_uses_next_31_seed_sequence() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Ljava/util/Random;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VJ".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["J".to_owned()],
        },
    };
    let method = DexMethodId {
        class_descriptor: "Ljava/util/Random;".to_owned(),
        name: "nextInt".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "II".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let random = DexValue::Reference(std::rc::Rc::from("Ljava/util/Random;:seed-zero"));
    let constructor_invocation = test_invocation(vec![random.clone(), DexValue::Long(0)]);
    let invocation = test_invocation(vec![random.clone(), DexValue::Int(65536)]);
    let invalid_bound = test_invocation(vec![random, DexValue::Int(0)]);

    assert_eq!(
        program.invoke_builtin_method(&constructor, Some(&constructor_invocation)),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Int(47904))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Int(54489))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invalid_bound)),
        None
    );
}

// cite: libcore Random.java#Random, nextInt, next
#[test]
fn libcore_random_next_int_no_arg_uses_next_32_seed_sequence() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Ljava/util/Random;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VJ".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["J".to_owned()],
        },
    };
    let next_int = DexMethodId {
        class_descriptor: "Ljava/util/Random;".to_owned(),
        name: "nextInt".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let random = DexValue::Reference(std::rc::Rc::from("Ljava/util/Random;:seed-zero"));
    let constructor_invocation = test_invocation(vec![random.clone(), DexValue::Long(0)]);
    let invocation = test_invocation(vec![random]);

    assert_eq!(
        program.invoke_builtin_method(&constructor, Some(&constructor_invocation)),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&next_int, Some(&invocation)),
        Some(DexExecutionResult::Int(-1155484576))
    );
}

// cite: libcore Random.java#Random, nextFloat, next
#[test]
fn libcore_random_next_float_uses_next_24_seed_sequence() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Ljava/util/Random;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VJ".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["J".to_owned()],
        },
    };
    let next_float = DexMethodId {
        class_descriptor: "Ljava/util/Random;".to_owned(),
        name: "nextFloat".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "F".to_owned(),
            return_type: "F".to_owned(),
            parameters: Vec::new(),
        },
    };
    let random = DexValue::Reference(std::rc::Rc::from("Ljava/util/Random;:float-seed-zero"));
    let constructor_invocation = test_invocation(vec![random.clone(), DexValue::Long(0)]);
    let invocation = test_invocation(vec![random]);

    assert_eq!(
        program.invoke_builtin_method(&constructor, Some(&constructor_invocation)),
        Some(DexExecutionResult::Void)
    );
    let expected = ((-1155484576_i32 as u32) >> 8) as f32 / 16777216.0;
    assert_eq!(
        program.invoke_builtin_method(&next_float, Some(&invocation)),
        Some(DexExecutionResult::Value(DexValue::Float(expected)))
    );
    assert!((0.0..1.0).contains(&expected));
}

// cite: libcore Pattern.java#quote
#[test]
fn pattern_quote_wraps_literal_and_escapes_embedded_terminator() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/util/regex/Pattern;".to_owned(),
        name: "quote".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned()],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![DexValue::String(std::rc::Rc::from(
                "a.b"
            ))]))
        ),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("\\Qa.b\\E")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![DexValue::String(std::rc::Rc::from(
                "a\\Eb"
            ))]))
        ),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("\\Qa\\E\\\\E\\Qb\\E")
        )))
    );
}

// cite: libcore Pattern.java#compile, CASE_INSENSITIVE, UNICODE_CASE, RemoveQEQuoting
#[test]
fn pattern_compile_with_flags_accepts_quoted_navigation_deep_link_pattern() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/util/regex/Pattern;".to_owned(),
        name: "compile".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LLI".to_owned(),
            return_type: "Ljava/util/regex/Pattern;".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned(), "I".to_owned()],
        },
    };

    let result = program.invoke_builtin_method(
        &method,
        Some(&test_invocation(vec![
            DexValue::String(std::rc::Rc::from(
                "^\\Qandroid-app://androidx.navigation/settings\\E($|(\\?(.)*)|(#(.)*))",
            )),
            DexValue::Int(0x42),
        ])),
    );

    assert!(matches!(
        result,
        Some(DexExecutionResult::Value(DexValue::Object(_)))
    ));
}

// cite: libcore Pattern.java#RemoveQEQuoting
#[test]
fn translate_java_regex_quoting_escapes_quoted_sections_only() {
    assert_eq!(translate_java_regex_quoting("a.b"), "a.b");
    assert_eq!(translate_java_regex_quoting("\\Qa.b\\Ec."), "a\\.bc.");
    assert_eq!(translate_java_regex_quoting("\\Qx(y\\E"), "x\\(y");
    assert_eq!(translate_java_regex_quoting("\\\\Q"), "\\\\Q");
}

#[test]
fn builtin_matcher_matches_forces_lazy_optional_group_to_span_whole_input() {
    // cite: libcore Matcher.java#matches
    let program = DexProgram::new(Vec::new());
    let compile = DexMethodId {
        class_descriptor: "Ljava/util/regex/Pattern;".to_owned(),
        name: "compile".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/util/regex/Pattern;".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned()],
        },
    };
    let matcher_method = DexMethodId {
        class_descriptor: "Ljava/util/regex/Pattern;".to_owned(),
        name: "matcher".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/util/regex/Matcher;".to_owned(),
            parameters: vec!["Ljava/lang/CharSequence;".to_owned()],
        },
    };
    let matches = DexMethodId {
        class_descriptor: "Ljava/util/regex/Matcher;".to_owned(),
        name: "matches".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };

    let Some(DexExecutionResult::Value(pattern)) = program.invoke_builtin_method(
        &compile,
        Some(&test_invocation(vec![DexValue::String(std::rc::Rc::from(
            "(.+?)?",
        ))])),
    ) else {
        panic!("Pattern.compile did not return a pattern");
    };
    let Some(DexExecutionResult::Value(matcher)) = program.invoke_builtin_method(
        &matcher_method,
        Some(&test_invocation(vec![
            pattern,
            DexValue::String(std::rc::Rc::from("-657931")),
        ])),
    ) else {
        panic!("Pattern.matcher did not return a matcher");
    };

    assert_eq!(
        program.invoke_builtin_method(&matches, Some(&test_invocation(vec![matcher]))),
        Some(DexExecutionResult::Int(1))
    );
}

#[test]
fn libcore_pattern_zero_width_lookahead_matches_and_splits() {
    // cite: libcore Pattern.java#compile
    // cite: libcore Matcher.java#matches, find
    let program = DexProgram::new(Vec::new());
    let compile = builtin_method(
        "Ljava/util/regex/Pattern;",
        "compile",
        "Ljava/util/regex/Pattern;",
        &["Ljava/lang/String;"],
    );
    let matcher_method = builtin_method(
        "Ljava/util/regex/Pattern;",
        "matcher",
        "Ljava/util/regex/Matcher;",
        &["Ljava/lang/CharSequence;"],
    );
    let matches = builtin_method("Ljava/util/regex/Matcher;", "matches", "Z", &[]);
    let find = builtin_method("Ljava/util/regex/Matcher;", "find", "Z", &[]);
    let text = |value: &str| DexValue::String(std::rc::Rc::from(value));
    let Some(DexExecutionResult::Value(pattern)) = program.invoke_builtin_method(
        &compile,
        Some(&test_invocation(vec![text(
            r"^(?=.{1,12}$)(?=.*@)[a-z]+@[a-z]+\.[a-z]{2,}$",
        )])),
    ) else {
        panic!("a lookahead pattern should compile");
    };
    let outcome = |input: &str, method: &DexMethodId| {
        let Some(DexExecutionResult::Value(matcher)) = program.invoke_builtin_method(
            &matcher_method,
            Some(&test_invocation(vec![pattern.clone(), text(input)])),
        ) else {
            panic!("Pattern.matcher should return a Matcher");
        };

        program.invoke_builtin_method(method, Some(&test_invocation(vec![matcher])))
    };

    assert_eq!(
        outcome("ab@cd.com", &matches),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        outcome("abcd.com", &matches),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        outcome("abcdefghij@kl.com", &find),
        Some(DexExecutionResult::Int(0))
    );

    let split = builtin_method(
        "Ljava/lang/String;",
        "split",
        "[Ljava/lang/String;",
        &["Ljava/lang/String;"],
    );
    let Some(DexExecutionResult::Value(parts)) = program.invoke_builtin_method(
        &split,
        Some(&test_invocation(vec![text("aXbXc"), text("(?<=X)")])),
    ) else {
        panic!("String.split with a lookbehind should run");
    };
    let read = |index| program.read_array_value(&parts, index);
    assert_eq!(program.array_length(&parts), Some(3));
    assert_eq!(read(0), Some(text("aX")));
    assert_eq!(read(1), Some(text("bX")));
    assert_eq!(read(2), Some(text("c")));
}

#[test]
fn libcore_pattern_backtracking_engine_error_is_a_missing_primitive_not_no_match() {
    // cite: libcore Pattern.java#compile
    // cite: libcore Matcher.java#find
    let program = DexProgram::new(Vec::new());
    let compile = builtin_method(
        "Ljava/util/regex/Pattern;",
        "compile",
        "Ljava/util/regex/Pattern;",
        &["Ljava/lang/String;"],
    );
    let matcher_method = builtin_method(
        "Ljava/util/regex/Pattern;",
        "matcher",
        "Ljava/util/regex/Matcher;",
        &["Ljava/lang/CharSequence;"],
    );
    let find = builtin_method("Ljava/util/regex/Matcher;", "find", "Z", &[]);
    let text = |value: &str| DexValue::String(std::rc::Rc::from(value));
    let Some(DexExecutionResult::Value(pattern)) = program.invoke_builtin_method(
        &compile,
        Some(&test_invocation(vec![text(r"(?=a)(a+)+\1$")])),
    ) else {
        panic!("a lookahead pattern should compile");
    };
    let Some(DexExecutionResult::Value(matcher)) = program.invoke_builtin_method(
        &matcher_method,
        Some(&test_invocation(vec![
            pattern,
            text(&format!("{}b", "a".repeat(64))),
        ])),
    ) else {
        panic!("Pattern.matcher should return a Matcher");
    };

    assert_eq!(
        program.invoke_builtin_method(&find, Some(&test_invocation(vec![matcher]))),
        None
    );
}
