use super::super::*;
use super::helpers::*;

#[test]
fn builtin_char_sequence_to_string_returns_the_underlying_text() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/CharSequence;".to_owned(),
        name: "toString".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: Vec::new(),
        },
    };

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![DexValue::String(std::rc::Rc::from(
                "13.000"
            ))]))
        ),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("13.000")
        )))
    );
}

#[test]
fn builtin_break_iterator_get_line_instance_allocates_line_iterator_for_locale() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/text/BreakIterator;".to_owned(),
        name: "getLineInstance".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/text/BreakIterator;".to_owned(),
            parameters: vec!["Ljava/util/Locale;".to_owned()],
        },
    };
    let locale = DexValue::Reference(std::rc::Rc::from("Ljava/util/Locale;:default"));

    let Some(DexExecutionResult::Value(iterator)) =
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![locale.clone()])))
    else {
        panic!("BreakIterator.getLineInstance(Locale) should allocate an iterator");
    };

    assert_eq!(
        value_class_descriptor(&iterator),
        Some("Ljava/text/BreakIterator;")
    );
    assert_eq!(
        program.read_stored_instance_field(&iterator, &break_iterator_string_field("kind")),
        Some(DexValue::String(std::rc::Rc::from("line")))
    );
    assert_eq!(
        program.read_stored_instance_field(
            &iterator,
            &break_iterator_object_field("locale", "Ljava/util/Locale;")
        ),
        Some(locale)
    );
}

#[test]
fn builtin_break_iterator_next_walks_stored_boundaries_until_done() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/text/BreakIterator;".to_owned(),
        name: "next".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let break_iterator = DexValue::Object(DexObjectRef {
        id: 7,
        class_descriptor: std::rc::Rc::from("Ljava/text/BreakIterator;"),
    });
    let boundaries = program.allocate_filled_array(
        "[I".to_owned(),
        vec![DexValue::Int(0), DexValue::Int(3), DexValue::Int(9)],
    );
    assert!(program.write_stored_instance_field(
        &break_iterator,
        &break_iterator_object_field("boundaries", "[I"),
        boundaries
    ));
    assert!(program.write_stored_instance_field(
        &break_iterator,
        &break_iterator_int_field("boundaryIndex"),
        DexValue::Int(0)
    ));

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![break_iterator.clone()]))
        ),
        Some(DexExecutionResult::Int(3))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![break_iterator.clone()]))
        ),
        Some(DexExecutionResult::Int(9))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![break_iterator]))),
        Some(DexExecutionResult::Int(-1))
    );
}

#[test]
fn builtin_string_value_of_char_returns_one_character_string() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/String;".to_owned(),
        name: "valueOf".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LC".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: vec!["C".to_owned()],
        },
    };
    let invocation = test_invocation(vec![DexValue::Int('.' as i32)]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from(".")
        )))
    );
}

#[test]
fn builtin_string_value_of_int_returns_decimal_string() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/String;".to_owned(),
        name: "valueOf".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LI".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![DexValue::Int(42)]))),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("42")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![DexValue::Int(-1)]))),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("-1")
        )))
    );
}

#[test]
fn builtin_string_value_of_long_returns_decimal_string() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/String;".to_owned(),
        name: "valueOf".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LJ".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: vec!["J".to_owned()],
        },
    };
    let invocation = test_invocation(vec![DexValue::Long(42), DexValue::Long(42)]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("42")
        )))
    );
}

#[test]
fn builtin_string_value_of_double_returns_decimal_string() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/String;".to_owned(),
        name: "valueOf".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LD".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: vec!["D".to_owned()],
        },
    };
    let bits = 12.5_f64.to_bits() as i64;
    let invocation = test_invocation(vec![DexValue::Long(bits), DexValue::Long(bits)]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("12.5")
        )))
    );
}

#[test]
fn builtin_string_value_of_object_returns_null_string_or_object_text() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/String;".to_owned(),
        name: "valueOf".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let object = DexValue::Object(DexObjectRef {
        id: 31,
        class_descriptor: std::rc::Rc::from("Lcom/example/Thing;"),
    });
    let object_text = format!(
        "com.example.Thing@{:x}",
        stable_hash_i32(&map_identity_key(&object)) as u32
    );

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![DexValue::Null]))),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("null")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![DexValue::String(std::rc::Rc::from(
                "ready"
            ))]))
        ),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("ready")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![object]))),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from(object_text)
        )))
    );
}

#[test]
fn builtin_string_format_formats_object_array_arguments() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/String;".to_owned(),
        name: "format".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LLL".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: vec![
                "Ljava/lang/String;".to_owned(),
                "[Ljava/lang/Object;".to_owned(),
            ],
        },
    };
    let method_with_locale = DexMethodId {
        class_descriptor: "Ljava/lang/String;".to_owned(),
        name: "format".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LLLL".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: vec![
                "Ljava/util/Locale;".to_owned(),
                "Ljava/lang/String;".to_owned(),
                "[Ljava/lang/Object;".to_owned(),
            ],
        },
    };
    let unicode_args = program.allocate_filled_array(
        "[Ljava/lang/Object;".to_owned(),
        vec![DexValue::Reference(std::rc::Rc::from(
            "Ljava/lang/Integer;:10",
        ))],
    );
    let mixed_args = program.allocate_filled_array(
        "[Ljava/lang/Object;".to_owned(),
        vec![
            DexValue::String(std::rc::Rc::from("value")),
            DexValue::Int(7),
            DexValue::Reference(std::rc::Rc::from("Ljava/lang/Integer;:15")),
        ],
    );

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::String(std::rc::Rc::from("\\u%04x")),
                unicode_args
            ]))
        ),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("\\u000a")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::String(std::rc::Rc::from("%s-%d-%04X")),
                mixed_args
            ]))
        ),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("value-7-000F")
        )))
    );
    let locale_args = program.allocate_filled_array(
        "[Ljava/lang/Object;".to_owned(),
        vec![DexValue::Int(3), DexValue::Int(4)],
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method_with_locale,
            Some(&test_invocation(vec![
                DexValue::Reference(std::rc::Rc::from("Ljava/util/Locale;:US")),
                DexValue::String(std::rc::Rc::from("%02d:%02d")),
                locale_args,
            ]))
        ),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("03:04")
        )))
    );
}

#[test]
fn builtin_string_constructor_reads_char_array_range() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Ljava/lang/String;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VLII".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["[C".to_owned(), "I".to_owned(), "I".to_owned()],
        },
    };
    let full_constructor = DexMethodId {
        class_descriptor: "Ljava/lang/String;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["[C".to_owned()],
        },
    };
    let to_string = DexMethodId {
        class_descriptor: "Ljava/lang/String;".to_owned(),
        name: "toString".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let length = DexMethodId {
        class_descriptor: "Ljava/lang/String;".to_owned(),
        name: "length".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let char_at = DexMethodId {
        class_descriptor: "Ljava/lang/String;".to_owned(),
        name: "charAt".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "CI".to_owned(),
            return_type: "C".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let equals = DexMethodId {
        class_descriptor: "Ljava/lang/String;".to_owned(),
        name: "equals".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let array = program.allocate_filled_array(
        "[C".to_owned(),
        "a\u{1f600}bc"
            .encode_utf16()
            .map(|unit| DexValue::Int(i32::from(unit)))
            .collect(),
    );
    let string = DexValue::Object(DexObjectRef {
        id: 7,
        class_descriptor: std::rc::Rc::from("Ljava/lang/String;"),
    });
    let full_string = DexValue::Object(DexObjectRef {
        id: 8,
        class_descriptor: std::rc::Rc::from("Ljava/lang/String;"),
    });

    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![
                string.clone(),
                array.clone(),
                DexValue::Int(1),
                DexValue::Int(3)
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &full_constructor,
            Some(&test_invocation(vec![full_string.clone(), array.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&to_string, Some(&test_invocation(vec![string.clone()]))),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("\u{1f600}b")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&to_string, Some(&test_invocation(vec![full_string]))),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("a\u{1f600}bc")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&length, Some(&test_invocation(vec![string.clone()]))),
        Some(DexExecutionResult::Int(3))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &char_at,
            Some(&test_invocation(vec![string.clone(), DexValue::Int(0)]))
        ),
        Some(DexExecutionResult::Int(0xd83d))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &equals,
            Some(&test_invocation(vec![
                string,
                DexValue::String(std::rc::Rc::from("\u{1f600}b"))
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
}

#[test]
fn builtin_string_equality_methods_compare_string_values() {
    let program = DexProgram::new(Vec::new());
    let equals = DexMethodId {
        class_descriptor: "Ljava/lang/String;".to_owned(),
        name: "equals".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let equals_ignore_case = DexMethodId {
        class_descriptor: "Ljava/lang/String;".to_owned(),
        name: "equalsIgnoreCase".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned()],
        },
    };
    let starts_with = DexMethodId {
        class_descriptor: "Ljava/lang/String;".to_owned(),
        name: "startsWith".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned()],
        },
    };
    let ends_with = DexMethodId {
        class_descriptor: "Ljava/lang/String;".to_owned(),
        name: "endsWith".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned()],
        },
    };
    let equals_invocation = test_invocation(vec![
        DexValue::String(std::rc::Rc::from("Settings")),
        DexValue::String(std::rc::Rc::from("Settings")),
    ]);
    let equals_ignore_case_invocation = test_invocation(vec![
        DexValue::String(std::rc::Rc::from("Settings")),
        DexValue::String(std::rc::Rc::from("settings")),
    ]);
    let starts_with_invocation = test_invocation(vec![
        DexValue::String(std::rc::Rc::from("7.000")),
        DexValue::String(std::rc::Rc::from("7.")),
    ]);
    let ends_with_invocation = test_invocation(vec![
        DexValue::String(std::rc::Rc::from("7+")),
        DexValue::String(std::rc::Rc::from("+")),
    ]);

    assert_eq!(
        program.invoke_builtin_method(&equals, Some(&equals_invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&equals_ignore_case, Some(&equals_ignore_case_invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&starts_with, Some(&starts_with_invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&ends_with, Some(&ends_with_invocation)),
        Some(DexExecutionResult::Int(1))
    );
}

#[test]
fn builtin_string_length_returns_utf16_code_unit_count() {
    let program = DexProgram::new(Vec::new());
    let length = DexMethodId {
        class_descriptor: "Ljava/lang/String;".to_owned(),
        name: "length".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let is_empty = DexMethodId {
        class_descriptor: "Ljava/lang/String;".to_owned(),
        name: "isEmpty".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let invocation = test_invocation(vec![DexValue::String(std::rc::Rc::from("country"))]);
    let empty_invocation =
        test_invocation(vec![DexValue::String(std::rc::Rc::from(String::new()))]);

    assert_eq!(
        program.invoke_builtin_method(&length, Some(&invocation)),
        Some(DexExecutionResult::Int(7))
    );
    assert_eq!(
        program.invoke_builtin_method(&is_empty, Some(&invocation)),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&is_empty, Some(&empty_invocation)),
        Some(DexExecutionResult::Int(1))
    );
}

#[test]
fn builtin_string_hash_code_uses_java_utf16_algorithm() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/String;".to_owned(),
        name: "hashCode".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![DexValue::String(std::rc::Rc::from(
                "Aa"
            ))]))
        ),
        Some(DexExecutionResult::Int(2112))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![DexValue::String(std::rc::Rc::from(
                "BB"
            ))]))
        ),
        Some(DexExecutionResult::Int(2112))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![DexValue::String(std::rc::Rc::from(
                "√9"
            ))]))
        ),
        Some(DexExecutionResult::Int(270687))
    );
}

#[test]
fn builtin_text_utils_is_empty_handles_char_sequences() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/text/TextUtils;".to_owned(),
        name: "isEmpty".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/CharSequence;".to_owned()],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![DexValue::Null]))),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![DexValue::Int(0)]))),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![DexValue::String(std::rc::Rc::from(
                String::new()
            ))]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![DexValue::String(std::rc::Rc::from(
                "title"
            ))]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![DexValue::Reference(
                std::rc::Rc::from("Ljava/lang/StringBuilder;:title")
            )]))
        ),
        Some(DexExecutionResult::Int(0))
    );
}

#[test]
fn builtin_text_utils_equals_handles_nulls_strings_and_references() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/text/TextUtils;".to_owned(),
        name: "equals".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZLL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec![
                "Ljava/lang/CharSequence;".to_owned(),
                "Ljava/lang/CharSequence;".to_owned(),
            ],
        },
    };
    let builder = DexValue::Reference(std::rc::Rc::from("Ljava/lang/StringBuilder;:title"));

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![DexValue::Null, DexValue::Int(0)]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::Null,
                DexValue::String(std::rc::Rc::from("title"))
            ]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::String(std::rc::Rc::from("title")),
                DexValue::String(std::rc::Rc::from("title"))
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![builder.clone(), builder]))
        ),
        Some(DexExecutionResult::Int(1))
    );
}

#[test]
fn builtin_string_substring_uses_utf16_code_unit_index() {
    let program = DexProgram::new(Vec::new());
    let substring_from = DexMethodId {
        class_descriptor: "Ljava/lang/String;".to_owned(),
        name: "substring".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LI".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let substring_range = DexMethodId {
        class_descriptor: "Ljava/lang/String;".to_owned(),
        name: "substring".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LII".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: vec!["I".to_owned(), "I".to_owned()],
        },
    };
    let from_invocation = test_invocation(vec![
        DexValue::String(std::rc::Rc::from("a\u{1f600}bc.def")),
        DexValue::Int(5),
    ]);
    let range_invocation = test_invocation(vec![
        DexValue::String(std::rc::Rc::from("a\u{1f600}bc.def")),
        DexValue::Int(5),
        DexValue::Int(8),
    ]);

    assert_eq!(
        program.invoke_builtin_method(&substring_from, Some(&from_invocation)),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from(".def")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&substring_range, Some(&range_invocation)),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from(".de")
        )))
    );
}

#[test]
fn builtin_string_sub_sequence_uses_utf16_code_unit_range() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/String;".to_owned(),
        name: "subSequence".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LII".to_owned(),
            return_type: "Ljava/lang/CharSequence;".to_owned(),
            parameters: vec!["I".to_owned(), "I".to_owned()],
        },
    };
    let invocation = test_invocation(vec![
        DexValue::String(std::rc::Rc::from("a\u{1f600}bc.def")),
        DexValue::Int(5),
        DexValue::Int(8),
    ]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from(".de")
        )))
    );
}

#[test]
fn builtin_string_char_at_uses_utf16_code_unit_index() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/String;".to_owned(),
        name: "charAt".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "CI".to_owned(),
            return_type: "C".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let invocation = test_invocation(vec![
        DexValue::String(std::rc::Rc::from("a\u{1f600}bc")),
        DexValue::Int(1),
    ]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Int(0xd83d))
    );
}

#[test]
fn builtin_string_index_of_uses_utf16_code_unit_indices() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/String;".to_owned(),
        name: "indexOf".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ILI".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned(), "I".to_owned()],
        },
    };
    let default_from_method = DexMethodId {
        class_descriptor: "Ljava/lang/String;".to_owned(),
        name: "indexOf".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "IL".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned()],
        },
    };
    let invocation = test_invocation(vec![
        DexValue::String(std::rc::Rc::from("a\u{1f600}bc.def")),
        DexValue::String(std::rc::Rc::from(".")),
        DexValue::Int(0),
    ]);
    let default_from_invocation = test_invocation(vec![
        DexValue::String(std::rc::Rc::from("a\u{1f600}bc.def")),
        DexValue::String(std::rc::Rc::from(".")),
    ]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Int(5))
    );
    assert_eq!(
        program.invoke_builtin_method(&default_from_method, Some(&default_from_invocation)),
        Some(DexExecutionResult::Int(5))
    );
}

#[test]
fn builtin_string_compare_to_ignore_case_ignores_case_and_falls_back_to_length() {
    // cite: libcore String.java#compareToIgnoreCase, compare
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/String;".to_owned(),
        name: "compareToIgnoreCase".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ILL".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned()],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::String(std::rc::Rc::from("Report.pdf")),
                DexValue::String(std::rc::Rc::from("REPORT.PDF")),
            ]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::String(std::rc::Rc::from("Apple")),
                DexValue::String(std::rc::Rc::from("banana")),
            ]))
        ),
        Some(DexExecutionResult::Int(i32::from(b'a') - i32::from(b'b')))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::String(std::rc::Rc::from("FILE")),
                DexValue::String(std::rc::Rc::from("file.txt")),
            ]))
        ),
        Some(DexExecutionResult::Int(-4))
    );
}

#[test]
fn builtin_string_last_index_of_string_uses_utf16_code_unit_indices() {
    // cite: libcore String.java#lastIndexOf
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/String;".to_owned(),
        name: "lastIndexOf".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ILI".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned(), "I".to_owned()],
        },
    };
    let default_from_method = DexMethodId {
        class_descriptor: "Ljava/lang/String;".to_owned(),
        name: "lastIndexOf".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "IL".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned()],
        },
    };
    let invocation = test_invocation(vec![
        DexValue::String(std::rc::Rc::from("a\u{1f600}bc.def.ghi")),
        DexValue::String(std::rc::Rc::from(".")),
        DexValue::Int(8),
    ]);
    let default_from_invocation = test_invocation(vec![
        DexValue::String(std::rc::Rc::from("a\u{1f600}bc.def.ghi")),
        DexValue::String(std::rc::Rc::from(".")),
    ]);
    let not_found_invocation = test_invocation(vec![
        DexValue::String(std::rc::Rc::from("abc")),
        DexValue::String(std::rc::Rc::from("z")),
    ]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Int(5))
    );
    assert_eq!(
        program.invoke_builtin_method(&default_from_method, Some(&default_from_invocation)),
        Some(DexExecutionResult::Int(9))
    );
    assert_eq!(
        program.invoke_builtin_method(&default_from_method, Some(&not_found_invocation)),
        Some(DexExecutionResult::Int(-1))
    );
}

#[test]
fn builtin_string_index_of_code_point_uses_utf16_code_unit_indices() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/String;".to_owned(),
        name: "indexOf".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "III".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["I".to_owned(), "I".to_owned()],
        },
    };
    let default_from_method = DexMethodId {
        class_descriptor: "Ljava/lang/String;".to_owned(),
        name: "indexOf".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "II".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let dot_invocation = test_invocation(vec![
        DexValue::String(std::rc::Rc::from("a\u{1f600}bc.def")),
        DexValue::Int(i32::from(b'.')),
        DexValue::Int(0),
    ]);
    let emoji_invocation = test_invocation(vec![
        DexValue::String(std::rc::Rc::from("a\u{1f600}bc.def")),
        DexValue::Int(0x1f600),
        DexValue::Int(0),
    ]);
    let default_from_dot_invocation = test_invocation(vec![
        DexValue::String(std::rc::Rc::from("a\u{1f600}bc.def")),
        DexValue::Int(i32::from(b'.')),
    ]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&dot_invocation)),
        Some(DexExecutionResult::Int(5))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&emoji_invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&default_from_method, Some(&default_from_dot_invocation)),
        Some(DexExecutionResult::Int(5))
    );
}

#[test]
fn builtin_string_last_index_of_uses_utf16_code_unit_indices() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/String;".to_owned(),
        name: "lastIndexOf".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "III".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["I".to_owned(), "I".to_owned()],
        },
    };
    let dot_invocation = test_invocation(vec![
        DexValue::String(std::rc::Rc::from("a\u{1f600}bc.def.ghi")),
        DexValue::Int(i32::from(b'.')),
        DexValue::Int(13),
    ]);
    let emoji_invocation = test_invocation(vec![
        DexValue::String(std::rc::Rc::from("a\u{1f600}bc.def.ghi")),
        DexValue::Int(0x1f600),
        DexValue::Int(13),
    ]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&dot_invocation)),
        Some(DexExecutionResult::Int(9))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&emoji_invocation)),
        Some(DexExecutionResult::Int(1))
    );
}

#[test]
fn builtin_string_last_index_of_char_defaults_from_index_to_the_end_of_string() {
    // cite: libcore String.java#lastIndexOf
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/String;".to_owned(),
        name: "lastIndexOf".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "II".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let invocation = test_invocation(vec![
        DexValue::String(std::rc::Rc::from("a.bc.def.ghi")),
        DexValue::Int(i32::from(b'.')),
    ]);
    let missing_invocation = test_invocation(vec![
        DexValue::String(std::rc::Rc::from("abc")),
        DexValue::Int(i32::from(b'.')),
    ]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Int(8))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&missing_invocation)),
        Some(DexExecutionResult::Int(-1))
    );
}

#[test]
fn builtin_string_contains_matches_libcore_string_contains_and_throws_npe_on_null() {
    // cite: libcore String.java#contains
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/String;".to_owned(),
        name: "contains".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Ljava/lang/CharSequence;".to_owned()],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::String(std::rc::Rc::from("notes.txt")),
                DexValue::String(std::rc::Rc::from("es.t")),
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::String(std::rc::Rc::from("notes.txt")),
                DexValue::String(std::rc::Rc::from("missing")),
            ]))
        ),
        Some(DexExecutionResult::Int(0))
    );

    let contains = |receiver: DexValue, needle: DexValue| {
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![receiver, needle])))
    };
    let text = || DexValue::String(std::rc::Rc::from("notes.txt"));

    for result in [
        contains(DexValue::Null, text()),
        contains(text(), DexValue::Null),
    ] {
        let Some(DexExecutionResult::Throw(exception)) = result else {
            panic!("null receiver or needle must throw");
        };
        assert!(program.value_matches_type(&exception, "Ljava/lang/NullPointerException;"));
    }

    let builder = program.allocate_object("Ljava/lang/StringBuilder;".to_owned());
    program
        .runtime
        .borrow_mut()
        .string_builder_values
        .insert(map_identity_key(&builder), "es.t".to_owned());
    assert_eq!(contains(text(), builder), Some(DexExecutionResult::Int(1)));
}

#[test]
fn builtin_string_split_comma_returns_stored_string_array() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/String;".to_owned(),
        name: "split".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LLI".to_owned(),
            return_type: "[Ljava/lang/String;".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned(), "I".to_owned()],
        },
    };
    let invocation = test_invocation(vec![
        DexValue::String(std::rc::Rc::from("en-US,fr-FR")),
        DexValue::String(std::rc::Rc::from(",")),
        DexValue::Int(-1),
    ]);

    let Some(DexExecutionResult::Value(array)) =
        program.invoke_builtin_method(&method, Some(&invocation))
    else {
        panic!("split should return an array");
    };

    assert_eq!(program.array_length(&array), Some(2));
    assert_eq!(
        program.read_array_value(&array, 0),
        Some(DexValue::String(std::rc::Rc::from("en-US")))
    );
    assert_eq!(
        program.read_array_value(&array, 1),
        Some(DexValue::String(std::rc::Rc::from("fr-FR")))
    );
}

#[test]
fn builtin_string_split_single_argument_removes_trailing_empty_parts_per_libcore_fast_split() {
    // cite: libcore Pattern.java#fastSplit
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/String;".to_owned(),
        name: "split".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "[Ljava/lang/String;".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned()],
        },
    };
    let invocation = test_invocation(vec![
        DexValue::String(std::rc::Rc::from("a,b,,,")),
        DexValue::String(std::rc::Rc::from(",")),
    ]);

    let Some(DexExecutionResult::Value(array)) =
        program.invoke_builtin_method(&method, Some(&invocation))
    else {
        panic!("split should return an array");
    };

    assert_eq!(program.array_length(&array), Some(2));
    assert_eq!(
        program.read_array_value(&array, 0),
        Some(DexValue::String(std::rc::Rc::from("a")))
    );
    assert_eq!(
        program.read_array_value(&array, 1),
        Some(DexValue::String(std::rc::Rc::from("b")))
    );

    let separators_only = test_invocation(vec![
        DexValue::String(std::rc::Rc::from(",,,")),
        DexValue::String(std::rc::Rc::from(",")),
    ]);
    let Some(DexExecutionResult::Value(empty_array)) =
        program.invoke_builtin_method(&method, Some(&separators_only))
    else {
        panic!("split should return an array");
    };
    assert_eq!(program.array_length(&empty_array), Some(0));

    let no_match = test_invocation(vec![
        DexValue::String(std::rc::Rc::from("abc")),
        DexValue::String(std::rc::Rc::from(",")),
    ]);
    let Some(DexExecutionResult::Value(whole_array)) =
        program.invoke_builtin_method(&method, Some(&no_match))
    else {
        panic!("split should return an array");
    };
    assert_eq!(program.array_length(&whole_array), Some(1));
    assert_eq!(
        program.read_array_value(&whole_array, 0),
        Some(DexValue::String(std::rc::Rc::from("abc")))
    );
}

#[test]
fn builtin_string_replace_chars_replaces_utf16_units() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/String;".to_owned(),
        name: "replace".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LCC".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: vec!["C".to_owned(), "C".to_owned()],
        },
    };
    let invocation = test_invocation(vec![
        DexValue::String(std::rc::Rc::from("7×8")),
        DexValue::Int('×' as i32),
        DexValue::Int('*' as i32),
    ]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("7*8")
        )))
    );
}

#[test]
fn builtin_string_concat_returns_combined_string() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/String;".to_owned(),
        name: "concat".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned()],
        },
    };
    let invocation = test_invocation(vec![
        DexValue::String(std::rc::Rc::from("left")),
        DexValue::String(std::rc::Rc::from("right")),
    ]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("leftright")
        )))
    );
}

#[test]
fn builtin_string_to_lower_case_locale_returns_lowercase_text() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/String;".to_owned(),
        name: "toLowerCase".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: vec!["Ljava/util/Locale;".to_owned()],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::String(std::rc::Rc::from("MeditationTimer")),
                DexValue::Reference(std::rc::Rc::from("Ljava/util/Locale;:US")),
            ])),
        ),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("meditationtimer")
        )))
    );
}

#[test]
fn builtin_string_to_upper_case_locale_returns_uppercase_text() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/String;".to_owned(),
        name: "toUpperCase".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: vec!["Ljava/util/Locale;".to_owned()],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::String(std::rc::Rc::from("MeditationTimer")),
                DexValue::Reference(std::rc::Rc::from("Ljava/util/Locale;:US")),
            ])),
        ),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("MEDITATIONTIMER")
        )))
    );
}

#[test]
fn builtin_string_to_lower_case_no_arg_returns_lowercase_text() {
    // cite: libcore String.java#toLowerCase
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/String;".to_owned(),
        name: "toLowerCase".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: Vec::new(),
        },
    };

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![DexValue::String(std::rc::Rc::from(
                "Report.PDF"
            ))])),
        ),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("report.pdf")
        )))
    );
}

#[test]
fn builtin_string_to_upper_case_no_arg_returns_uppercase_text() {
    // cite: libcore String.java#toUpperCase
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/String;".to_owned(),
        name: "toUpperCase".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: Vec::new(),
        },
    };

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![DexValue::String(std::rc::Rc::from(
                "report.pdf"
            ))])),
        ),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("REPORT.PDF")
        )))
    );
}

#[test]
fn builtin_char_sequence_sub_sequence_uses_utf16_code_unit_indices() {
    let program = DexProgram::new(Vec::new());
    let sub_sequence = DexMethodId {
        class_descriptor: "Ljava/lang/CharSequence;".to_owned(),
        name: "subSequence".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LII".to_owned(),
            return_type: "Ljava/lang/CharSequence;".to_owned(),
            parameters: vec!["I".to_owned(), "I".to_owned()],
        },
    };
    let length = DexMethodId {
        class_descriptor: "Ljava/lang/CharSequence;".to_owned(),
        name: "length".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let char_at = DexMethodId {
        class_descriptor: "Ljava/lang/CharSequence;".to_owned(),
        name: "charAt".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "CI".to_owned(),
            return_type: "C".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let sub_sequence_invocation = test_invocation(vec![
        DexValue::String(std::rc::Rc::from("a\u{1f600}bc")),
        DexValue::Int(3),
        DexValue::Int(5),
    ]);
    let length_invocation =
        test_invocation(vec![DexValue::String(std::rc::Rc::from("a\u{1f600}bc"))]);
    let char_at_invocation = test_invocation(vec![
        DexValue::String(std::rc::Rc::from("a\u{1f600}bc")),
        DexValue::Int(1),
    ]);

    assert_eq!(
        program.invoke_builtin_method(&sub_sequence, Some(&sub_sequence_invocation)),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("bc")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&length, Some(&length_invocation)),
        Some(DexExecutionResult::Int(5))
    );
    assert_eq!(
        program.invoke_builtin_method(&char_at, Some(&char_at_invocation)),
        Some(DexExecutionResult::Int(0xd83d))
    );
}

#[test]
fn builtin_string_builder_append_code_point_appends_valid_unicode_scalar() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Ljava/lang/StringBuilder;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let append_code_point = DexMethodId {
        class_descriptor: "Ljava/lang/StringBuilder;".to_owned(),
        name: "appendCodePoint".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LI".to_owned(),
            return_type: "Ljava/lang/StringBuilder;".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let to_string = DexMethodId {
        class_descriptor: "Ljava/lang/StringBuilder;".to_owned(),
        name: "toString".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let builder = DexValue::Object(DexObjectRef {
        id: 22,
        class_descriptor: std::rc::Rc::from("Ljava/lang/StringBuilder;"),
    });

    assert_eq!(
        program.invoke_builtin_method(&constructor, Some(&test_invocation(vec![builder.clone()]))),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &append_code_point,
            Some(&test_invocation(vec![
                builder.clone(),
                DexValue::Int('A' as i32)
            ]))
        ),
        Some(DexExecutionResult::Value(builder.clone()))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &append_code_point,
            Some(&test_invocation(vec![
                builder.clone(),
                DexValue::Int(0x1f600)
            ]))
        ),
        Some(DexExecutionResult::Value(builder.clone()))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &append_code_point,
            Some(&test_invocation(vec![
                builder.clone(),
                DexValue::Int(0xd800)
            ]))
        ),
        None
    );
    assert_eq!(
        program.invoke_builtin_method(&to_string, Some(&test_invocation(vec![builder]))),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("A\u{1f600}")
        )))
    );
}

#[test]
fn builtin_string_builder_methods_return_basic_values() {
    let program = DexProgram::new(Vec::new());
    let default_constructor = DexMethodId {
        class_descriptor: "Ljava/lang/StringBuilder;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let string_constructor = DexMethodId {
        class_descriptor: "Ljava/lang/StringBuilder;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned()],
        },
    };
    let char_sequence_constructor = DexMethodId {
        class_descriptor: "Ljava/lang/StringBuilder;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/lang/CharSequence;".to_owned()],
        },
    };
    let capacity_constructor = DexMethodId {
        class_descriptor: "Ljava/lang/StringBuilder;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let append = DexMethodId {
        class_descriptor: "Ljava/lang/StringBuilder;".to_owned(),
        name: "append".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/lang/StringBuilder;".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned()],
        },
    };
    let append_object = DexMethodId {
        class_descriptor: "Ljava/lang/StringBuilder;".to_owned(),
        name: "append".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/lang/StringBuilder;".to_owned(),
            parameters: vec!["Ljava/lang/Object;".to_owned()],
        },
    };
    let append_char_sequence_as_appendable = DexMethodId {
        class_descriptor: "Ljava/lang/StringBuilder;".to_owned(),
        name: "append".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/lang/Appendable;".to_owned(),
            parameters: vec!["Ljava/lang/CharSequence;".to_owned()],
        },
    };
    let append_char_sequence = DexMethodId {
        class_descriptor: "Ljava/lang/StringBuilder;".to_owned(),
        name: "append".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/lang/StringBuilder;".to_owned(),
            parameters: vec!["Ljava/lang/CharSequence;".to_owned()],
        },
    };
    let append_char_sequence_range = DexMethodId {
        class_descriptor: "Ljava/lang/StringBuilder;".to_owned(),
        name: "append".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LLII".to_owned(),
            return_type: "Ljava/lang/StringBuilder;".to_owned(),
            parameters: vec![
                "Ljava/lang/CharSequence;".to_owned(),
                "I".to_owned(),
                "I".to_owned(),
            ],
        },
    };
    let append_char = DexMethodId {
        class_descriptor: "Ljava/lang/StringBuilder;".to_owned(),
        name: "append".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LC".to_owned(),
            return_type: "Ljava/lang/StringBuilder;".to_owned(),
            parameters: vec!["C".to_owned()],
        },
    };
    let append_boolean = DexMethodId {
        class_descriptor: "Ljava/lang/StringBuilder;".to_owned(),
        name: "append".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LZ".to_owned(),
            return_type: "Ljava/lang/StringBuilder;".to_owned(),
            parameters: vec!["Z".to_owned()],
        },
    };
    let append_float = DexMethodId {
        class_descriptor: "Ljava/lang/StringBuilder;".to_owned(),
        name: "append".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LF".to_owned(),
            return_type: "Ljava/lang/StringBuilder;".to_owned(),
            parameters: vec!["F".to_owned()],
        },
    };
    let insert_char = DexMethodId {
        class_descriptor: "Ljava/lang/StringBuilder;".to_owned(),
        name: "insert".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LIC".to_owned(),
            return_type: "Ljava/lang/StringBuilder;".to_owned(),
            parameters: vec!["I".to_owned(), "C".to_owned()],
        },
    };
    let reverse = DexMethodId {
        class_descriptor: "Ljava/lang/StringBuilder;".to_owned(),
        name: "reverse".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/StringBuilder;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let to_string = DexMethodId {
        class_descriptor: "Ljava/lang/StringBuilder;".to_owned(),
        name: "toString".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let object_to_string = DexMethodId {
        class_descriptor: "Ljava/lang/Object;".to_owned(),
        name: "toString".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let builder = DexValue::Object(DexObjectRef {
        id: 7,
        class_descriptor: std::rc::Rc::from("Ljava/lang/StringBuilder;"),
    });
    let object_argument = DexValue::Object(DexObjectRef {
        id: 23,
        class_descriptor: std::rc::Rc::from("Lcom/example/Thing;"),
    });
    let object_argument_text = format!(
        "com.example.Thing@{:x}",
        stable_hash_i32(&map_identity_key(&object_argument)) as u32
    );
    let default_constructor_invocation = test_invocation(vec![builder.clone()]);
    let string_constructor_invocation = test_invocation(vec![
        builder.clone(),
        DexValue::String(std::rc::Rc::from("seed")),
    ]);
    let capacity_constructor_invocation = test_invocation(vec![builder.clone(), DexValue::Int(32)]);
    let append_invocation = DexInvocation {
        kind: DexInvokeKind::Virtual,
        range: false,
        method_index: 0,
        argument_registers: smallvec::smallvec![0, 1],
        arguments: smallvec::smallvec![builder.clone(), DexValue::String(std::rc::Rc::from("-x"))],
    };
    let append_object_invocation = DexInvocation {
        kind: DexInvokeKind::Virtual,
        range: false,
        method_index: 0,
        argument_registers: smallvec::smallvec![0, 1],
        arguments: smallvec::smallvec![builder.clone(), object_argument],
    };
    let append_char_sequence_as_appendable_invocation = DexInvocation {
        kind: DexInvokeKind::Virtual,
        range: false,
        method_index: 0,
        argument_registers: smallvec::smallvec![0, 1],
        arguments: smallvec::smallvec![builder.clone(), DexValue::String(std::rc::Rc::from("cs"))],
    };
    let append_char_sequence_invocation = DexInvocation {
        kind: DexInvokeKind::Virtual,
        range: false,
        method_index: 0,
        argument_registers: smallvec::smallvec![0, 1],
        arguments: smallvec::smallvec![builder.clone(), DexValue::String(std::rc::Rc::from("sb"))],
    };
    let append_char_sequence_range_invocation = DexInvocation {
        kind: DexInvokeKind::Virtual,
        range: false,
        method_index: 0,
        argument_registers: smallvec::smallvec![0, 1, 2, 3],
        arguments: smallvec::smallvec![
            builder.clone(),
            DexValue::String(std::rc::Rc::from("a\u{1f600}bc")),
            DexValue::Int(1),
            DexValue::Int(3),
        ],
    };
    let append_char_invocation = DexInvocation {
        kind: DexInvokeKind::Virtual,
        range: false,
        method_index: 0,
        argument_registers: smallvec::smallvec![0, 1],
        arguments: smallvec::smallvec![builder.clone(), DexValue::Int('_' as i32)],
    };
    let append_boolean_invocation = DexInvocation {
        kind: DexInvokeKind::Virtual,
        range: false,
        method_index: 0,
        argument_registers: smallvec::smallvec![0, 1],
        arguments: smallvec::smallvec![builder.clone(), DexValue::Int(1)],
    };
    let append_float_invocation = DexInvocation {
        kind: DexInvokeKind::Virtual,
        range: false,
        method_index: 0,
        argument_registers: smallvec::smallvec![0, 1],
        arguments: smallvec::smallvec![builder.clone(), DexValue::Float(1.5)],
    };
    let insert_char_invocation = DexInvocation {
        kind: DexInvokeKind::Virtual,
        range: false,
        method_index: 0,
        argument_registers: smallvec::smallvec![0, 1, 2],
        arguments: smallvec::smallvec![
            builder.clone(),
            DexValue::Int(3),
            DexValue::Int('!' as i32)
        ],
    };
    let to_string_invocation = test_invocation(vec![builder.clone()]);

    assert_eq!(
        program.invoke_builtin_method(&default_constructor, Some(&default_constructor_invocation)),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&to_string, Some(&to_string_invocation)),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from(String::new())
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &capacity_constructor,
            Some(&capacity_constructor_invocation)
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&to_string, Some(&to_string_invocation)),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from(String::new())
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&string_constructor, Some(&string_constructor_invocation)),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &char_sequence_constructor,
            Some(&test_invocation(vec![
                builder.clone(),
                DexValue::String(std::rc::Rc::from("char-seed"))
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&to_string, Some(&to_string_invocation)),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("char-seed")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&string_constructor, Some(&string_constructor_invocation)),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&append, Some(&append_invocation)),
        Some(DexExecutionResult::Value(builder.clone()))
    );
    assert_eq!(
        program.invoke_builtin_method(&append_object, Some(&append_object_invocation)),
        Some(DexExecutionResult::Value(builder.clone()))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &append_char_sequence_as_appendable,
            Some(&append_char_sequence_as_appendable_invocation)
        ),
        Some(DexExecutionResult::Value(builder.clone()))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &append_char_sequence,
            Some(&append_char_sequence_invocation)
        ),
        Some(DexExecutionResult::Value(builder.clone()))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &append_char_sequence_range,
            Some(&append_char_sequence_range_invocation)
        ),
        Some(DexExecutionResult::Value(builder.clone()))
    );
    assert_eq!(
        program.invoke_builtin_method(&append_char, Some(&append_char_invocation)),
        Some(DexExecutionResult::Value(builder.clone()))
    );
    assert_eq!(
        program.invoke_builtin_method(&append_boolean, Some(&append_boolean_invocation)),
        Some(DexExecutionResult::Value(builder.clone()))
    );
    assert_eq!(
        program.invoke_builtin_method(&append_float, Some(&append_float_invocation)),
        Some(DexExecutionResult::Value(builder.clone()))
    );
    assert_eq!(
        program.invoke_builtin_method(&insert_char, Some(&insert_char_invocation)),
        Some(DexExecutionResult::Value(builder.clone()))
    );
    assert_eq!(
        program.invoke_builtin_method(&to_string, Some(&to_string_invocation)),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from(format!(
                "see!d-x{object_argument_text}cssb\u{1f600}_true1.5"
            ))
        )))
    );
    let reversed_text: String = format!("see!d-x{object_argument_text}cssb\u{1f600}_true1.5")
        .chars()
        .rev()
        .collect();
    assert_eq!(
        program.invoke_builtin_method(&reverse, Some(&to_string_invocation)),
        Some(DexExecutionResult::Value(builder.clone()))
    );
    assert_eq!(
        program.invoke_builtin_method(&to_string, Some(&to_string_invocation)),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from(reversed_text.clone())
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&object_to_string, Some(&to_string_invocation)),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from(reversed_text)
        )))
    );
}

#[test]
fn builtin_string_builder_append_char_as_appendable_returns_builder() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Ljava/lang/StringBuilder;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let append_char_as_appendable = DexMethodId {
        class_descriptor: "Ljava/lang/StringBuilder;".to_owned(),
        name: "append".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LC".to_owned(),
            return_type: "Ljava/lang/Appendable;".to_owned(),
            parameters: vec!["C".to_owned()],
        },
    };
    let to_string = DexMethodId {
        class_descriptor: "Ljava/lang/StringBuilder;".to_owned(),
        name: "toString".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let builder = DexValue::Object(DexObjectRef {
        id: 71,
        class_descriptor: std::rc::Rc::from("Ljava/lang/StringBuilder;"),
    });

    assert_eq!(
        program.invoke_builtin_method(&constructor, Some(&test_invocation(vec![builder.clone()]))),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &append_char_as_appendable,
            Some(&test_invocation(vec![
                builder.clone(),
                DexValue::Int('x' as i32)
            ]))
        ),
        Some(DexExecutionResult::Value(builder.clone()))
    );
    assert_eq!(
        program.invoke_builtin_method(&to_string, Some(&test_invocation(vec![builder]))),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("x")
        )))
    );
}

#[test]
fn builtin_string_builder_length_returns_utf16_code_units() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Ljava/lang/StringBuilder;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned()],
        },
    };
    let length = DexMethodId {
        class_descriptor: "Ljava/lang/StringBuilder;".to_owned(),
        name: "length".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let builder = DexValue::Object(DexObjectRef {
        id: 72,
        class_descriptor: std::rc::Rc::from("Ljava/lang/StringBuilder;"),
    });

    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![
                builder.clone(),
                DexValue::String(std::rc::Rc::from("x\u{1f600}"))
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&length, Some(&test_invocation(vec![builder]))),
        Some(DexExecutionResult::Int(3))
    );
}

#[test]
fn builtin_object_to_string_formats_boxed_wrapper_values() {
    // cite: libcore Integer.java#toString
    // cite: libcore Long.java#toString
    let program = DexProgram::new(Vec::new());
    let object_to_string = DexMethodId {
        class_descriptor: "Ljava/lang/Object;".to_owned(),
        name: "toString".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let value_of = DexMethodId {
        class_descriptor: "Ljava/lang/Integer;".to_owned(),
        name: "valueOf".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LI".to_owned(),
            return_type: "Ljava/lang/Integer;".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };

    let Some(DexExecutionResult::Value(boxed)) =
        program.invoke_builtin_method(&value_of, Some(&test_invocation(vec![DexValue::Int(1)])))
    else {
        panic!("Integer.valueOf did not return a boxed reference");
    };

    assert_eq!(
        program.invoke_builtin_method(&object_to_string, Some(&test_invocation(vec![boxed]))),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("1")
        )))
    );
}

#[test]
fn builtin_string_builder_append_long_formats_decimal_value() {
    // cite: libcore AbstractStringBuilder.java#append
    let program = DexProgram::new(Vec::new());
    let builder = DexValue::Object(DexObjectRef {
        id: 901,
        class_descriptor: std::rc::Rc::from("Ljava/lang/StringBuilder;"),
    });
    let append_long = DexMethodId {
        class_descriptor: "Ljava/lang/StringBuilder;".to_owned(),
        name: "append".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LJ".to_owned(),
            return_type: "Ljava/lang/StringBuilder;".to_owned(),
            parameters: vec!["J".to_owned()],
        },
    };
    let to_string = DexMethodId {
        class_descriptor: "Ljava/lang/StringBuilder;".to_owned(),
        name: "toString".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: Vec::new(),
        },
    };

    assert_eq!(
        program.invoke_builtin_method(
            &append_long,
            Some(&test_invocation(vec![
                builder.clone(),
                DexValue::Long(-42),
                DexValue::Long(-42),
            ]))
        ),
        Some(DexExecutionResult::Value(builder.clone()))
    );
    assert_eq!(
        program.invoke_builtin_method(&to_string, Some(&test_invocation(vec![builder]))),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("-42")
        )))
    );
}
