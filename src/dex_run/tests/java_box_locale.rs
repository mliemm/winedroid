use super::super::*;
use super::helpers::*;

#[test]
fn builtin_boolean_value_of_round_trips_boolean_value() {
    let program = DexProgram::new(Vec::new());
    let value_of = DexMethodId {
        class_descriptor: "Ljava/lang/Boolean;".to_owned(),
        name: "valueOf".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LZ".to_owned(),
            return_type: "Ljava/lang/Boolean;".to_owned(),
            parameters: vec!["Z".to_owned()],
        },
    };
    let boolean_value = DexMethodId {
        class_descriptor: "Ljava/lang/Boolean;".to_owned(),
        name: "booleanValue".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let value_of_invocation = test_invocation(vec![DexValue::Int(1)]);
    let boolean_value_invocation = test_invocation(vec![DexValue::Reference(std::rc::Rc::from(
        "Ljava/lang/Boolean;:true",
    ))]);
    let true_field = DexFieldId {
        class_descriptor: "Ljava/lang/Boolean;".to_owned(),
        type_descriptor: "Ljava/lang/Boolean;".to_owned(),
        name: "TRUE".to_owned(),
    };
    let false_field = DexFieldId {
        class_descriptor: "Ljava/lang/Boolean;".to_owned(),
        type_descriptor: "Ljava/lang/Boolean;".to_owned(),
        name: "FALSE".to_owned(),
    };

    assert_eq!(
        program.invoke_builtin_method(&value_of, Some(&value_of_invocation)),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Ljava/lang/Boolean;:true")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&boolean_value, Some(&boolean_value_invocation)),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        bootstrap_static_field(&true_field),
        Some(DexValue::Reference(std::rc::Rc::from(
            "Ljava/lang/Boolean;:true"
        )))
    );
    assert_eq!(
        bootstrap_static_field(&false_field),
        Some(DexValue::Reference(std::rc::Rc::from(
            "Ljava/lang/Boolean;:false"
        )))
    );
}

#[test]
fn builtin_integer_value_of_returns_boxed_integer_reference() {
    let program = DexProgram::new(Vec::new());
    let value_of = DexMethodId {
        class_descriptor: "Ljava/lang/Integer;".to_owned(),
        name: "valueOf".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LI".to_owned(),
            return_type: "Ljava/lang/Integer;".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let int_value = DexMethodId {
        class_descriptor: "Ljava/lang/Integer;".to_owned(),
        name: "intValue".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let number_int_value = DexMethodId {
        class_descriptor: "Ljava/lang/Number;".to_owned(),
        name: "intValue".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let parse_int = DexMethodId {
        class_descriptor: "Ljava/lang/Integer;".to_owned(),
        name: "parseInt".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "IL".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned()],
        },
    };
    let parse_int_with_radix = DexMethodId {
        class_descriptor: "Ljava/lang/Integer;".to_owned(),
        name: "parseInt".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ILI".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned(), "I".to_owned()],
        },
    };
    let to_hex_string = DexMethodId {
        class_descriptor: "Ljava/lang/Integer;".to_owned(),
        name: "toHexString".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LI".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let invocation = test_invocation(vec![DexValue::Int(42)]);
    let int_value_invocation = test_invocation(vec![DexValue::Reference(std::rc::Rc::from(
        "Ljava/lang/Integer;:42",
    ))]);
    let parse_invocation = test_invocation(vec![DexValue::String(std::rc::Rc::from("-42"))]);
    let radix_invocation = test_invocation(vec![
        DexValue::String(std::rc::Rc::from("2a")),
        DexValue::Int(16),
    ]);

    assert_eq!(
        program.invoke_builtin_method(&value_of, Some(&invocation)),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Ljava/lang/Integer;:42")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&int_value, Some(&int_value_invocation)),
        Some(DexExecutionResult::Int(42))
    );
    assert_eq!(
        program.invoke_builtin_method(&number_int_value, Some(&int_value_invocation)),
        Some(DexExecutionResult::Int(42))
    );
    assert_eq!(
        program.invoke_builtin_method(&parse_int, Some(&parse_invocation)),
        Some(DexExecutionResult::Int(-42))
    );
    assert_eq!(
        program.invoke_builtin_method(&parse_int_with_radix, Some(&radix_invocation)),
        Some(DexExecutionResult::Int(42))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &to_hex_string,
            Some(&test_invocation(vec![DexValue::Int(26)]))
        ),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("1a")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &to_hex_string,
            Some(&test_invocation(vec![DexValue::Int(0)]))
        ),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("0")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &to_hex_string,
            Some(&test_invocation(vec![DexValue::Int(-1)]))
        ),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("ffffffff")
        )))
    );
}

#[test]
fn builtin_integer_constructor_stores_real_object_value() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Ljava/lang/Integer;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let number_int_value = DexMethodId {
        class_descriptor: "Ljava/lang/Number;".to_owned(),
        name: "intValue".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let object = DexValue::Object(DexObjectRef {
        id: 12,
        class_descriptor: std::rc::Rc::from("Ljava/lang/Integer;"),
    });
    let value_field = DexFieldId {
        class_descriptor: "Ljava/lang/Integer;".to_owned(),
        type_descriptor: "I".to_owned(),
        name: "value".to_owned(),
    };

    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![object.clone(), DexValue::Int(42)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&object, &value_field),
        Some(DexValue::Int(42))
    );
    assert_eq!(
        program.invoke_builtin_method(&number_int_value, Some(&test_invocation(vec![object]))),
        Some(DexExecutionResult::Int(42))
    );
}

#[test]
fn builtin_character_digit_uses_radix() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Character;".to_owned(),
        name: "digit".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "III".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["I".to_owned(), "I".to_owned()],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::Int('7' as i32),
                DexValue::Int(10)
            ]))
        ),
        Some(DexExecutionResult::Int(7))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::Int('f' as i32),
                DexValue::Int(16)
            ]))
        ),
        Some(DexExecutionResult::Int(15))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::Int('g' as i32),
                DexValue::Int(16)
            ]))
        ),
        Some(DexExecutionResult::Int(-1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::Int('7' as i32),
                DexValue::Int(1)
            ]))
        ),
        Some(DexExecutionResult::Int(-1))
    );
}

#[test]
fn builtin_character_value_of_returns_boxed_character_reference() {
    let program = DexProgram::new(Vec::new());
    let value_of = DexMethodId {
        class_descriptor: "Ljava/lang/Character;".to_owned(),
        name: "valueOf".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LC".to_owned(),
            return_type: "Ljava/lang/Character;".to_owned(),
            parameters: vec!["C".to_owned()],
        },
    };
    let char_value = DexMethodId {
        class_descriptor: "Ljava/lang/Character;".to_owned(),
        name: "charValue".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "C".to_owned(),
            return_type: "C".to_owned(),
            parameters: Vec::new(),
        },
    };

    assert_eq!(
        program.invoke_builtin_method(
            &value_of,
            Some(&test_invocation(vec![DexValue::Int('a' as i32)]))
        ),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Ljava/lang/Character;:97")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &char_value,
            Some(&test_invocation(vec![DexValue::Reference(
                std::rc::Rc::from("Ljava/lang/Character;:97")
            )]))
        ),
        Some(DexExecutionResult::Int('a' as i32))
    );
    assert_eq!(
        program.invoke_builtin_method(&value_of, Some(&test_invocation(vec![DexValue::Int(-1)]))),
        None
    );
    assert_eq!(
        program.invoke_builtin_method(
            &char_value,
            Some(&test_invocation(vec![DexValue::Reference(
                std::rc::Rc::from("Ljava/lang/Character;:65536")
            )]))
        ),
        None
    );
}

#[test]
fn builtin_character_code_point_at_reads_char_sequence_utf16_units() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Character;".to_owned(),
        name: "codePointAt".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ILI".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["Ljava/lang/CharSequence;".to_owned(), "I".to_owned()],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::String(std::rc::Rc::from("a\u{1f600}b")),
                DexValue::Int(0),
            ]))
        ),
        Some(DexExecutionResult::Int('a' as i32))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::String(std::rc::Rc::from("a\u{1f600}b")),
                DexValue::Int(1),
            ]))
        ),
        Some(DexExecutionResult::Int(0x1f600))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::String(std::rc::Rc::from("a\u{1f600}b")),
                DexValue::Int(2),
            ]))
        ),
        Some(DexExecutionResult::Int(0xde00))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::String(std::rc::Rc::from("a\u{1f600}b")),
                DexValue::Int(4),
            ]))
        ),
        None
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![DexValue::Null, DexValue::Int(0)]))
        ),
        None
    );
}

#[test]
fn builtin_character_char_count_matches_java_supplementary_rule() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Character;".to_owned(),
        name: "charCount".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "II".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![DexValue::Int(0)]))),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![DexValue::Int(0xffff)]))),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![DexValue::Int(0x10000)]))
        ),
        Some(DexExecutionResult::Int(2))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![DexValue::Int(-1)]))),
        Some(DexExecutionResult::Int(1))
    );
}

#[test]
fn builtin_character_is_digit_checks_decimal_chars() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Character;".to_owned(),
        name: "isDigit".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZI".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["C".to_owned()],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![DexValue::Int('7' as i32)]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![DexValue::Int('.' as i32)]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![DexValue::Int(-1)]))),
        Some(DexExecutionResult::Int(0))
    );
}

#[test]
fn builtin_character_is_whitespace_checks_space_chars() {
    let program = DexProgram::new(Vec::new());
    let whitespace = DexMethodId {
        class_descriptor: "Ljava/lang/Character;".to_owned(),
        name: "isWhitespace".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZC".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["C".to_owned()],
        },
    };
    let space_char = DexMethodId {
        class_descriptor: "Ljava/lang/Character;".to_owned(),
        name: "isSpaceChar".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZC".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["C".to_owned()],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(
            &whitespace,
            Some(&test_invocation(vec![DexValue::Int(' ' as i32)]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &whitespace,
            Some(&test_invocation(vec![DexValue::Int('a' as i32)]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&whitespace, Some(&test_invocation(vec![DexValue::Int(-1)]))),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &space_char,
            Some(&test_invocation(vec![DexValue::Int(0x2029)]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &space_char,
            Some(&test_invocation(vec![DexValue::Int('\n' as i32)]))
        ),
        Some(DexExecutionResult::Int(0))
    );
}

#[test]
fn builtin_long_value_of_returns_boxed_long_reference() {
    let program = DexProgram::new(Vec::new());
    let value_of = DexMethodId {
        class_descriptor: "Ljava/lang/Long;".to_owned(),
        name: "valueOf".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LJ".to_owned(),
            return_type: "Ljava/lang/Long;".to_owned(),
            parameters: vec!["J".to_owned()],
        },
    };
    let invocation = test_invocation(vec![DexValue::Long(42), DexValue::Long(42)]);

    assert_eq!(
        program.invoke_builtin_method(&value_of, Some(&invocation)),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Ljava/lang/Long;:42")
        )))
    );
}

#[test]
fn builtin_long_constructor_stores_object_value() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Ljava/lang/Long;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VJ".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["J".to_owned()],
        },
    };
    let long_value = DexMethodId {
        class_descriptor: "Ljava/lang/Long;".to_owned(),
        name: "longValue".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "J".to_owned(),
            return_type: "J".to_owned(),
            parameters: Vec::new(),
        },
    };
    let boxed = DexValue::Object(DexObjectRef {
        id: 91,
        class_descriptor: std::rc::Rc::from("Ljava/lang/Long;"),
    });

    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![
                boxed.clone(),
                DexValue::Long(77),
                DexValue::Long(77)
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&long_value, Some(&test_invocation(vec![boxed]))),
        Some(DexExecutionResult::Value(DexValue::Long(77)))
    );
}

#[test]
fn builtin_long_to_string_formats_radix_values() {
    let program = DexProgram::new(Vec::new());
    let decimal = DexMethodId {
        class_descriptor: "Ljava/lang/Long;".to_owned(),
        name: "toString".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LJ".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: vec!["J".to_owned()],
        },
    };
    let radix = DexMethodId {
        class_descriptor: "Ljava/lang/Long;".to_owned(),
        name: "toString".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LJI".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: vec!["J".to_owned(), "I".to_owned()],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(
            &decimal,
            Some(&test_invocation(vec![
                DexValue::Long(-42),
                DexValue::Long(-42)
            ]))
        ),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("-42")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &radix,
            Some(&test_invocation(vec![
                DexValue::Long(35),
                DexValue::Long(35),
                DexValue::Int(36)
            ]))
        ),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("z")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &radix,
            Some(&test_invocation(vec![
                DexValue::Long(10),
                DexValue::Long(10),
                DexValue::Int(1)
            ]))
        ),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("10")
        )))
    );
}

#[test]
fn builtin_float_value_of_returns_boxed_float_reference() {
    let program = DexProgram::new(Vec::new());
    let value_of = DexMethodId {
        class_descriptor: "Ljava/lang/Float;".to_owned(),
        name: "valueOf".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LF".to_owned(),
            return_type: "Ljava/lang/Float;".to_owned(),
            parameters: vec!["F".to_owned()],
        },
    };
    let float_value = DexMethodId {
        class_descriptor: "Ljava/lang/Float;".to_owned(),
        name: "floatValue".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "F".to_owned(),
            return_type: "F".to_owned(),
            parameters: Vec::new(),
        },
    };
    let number_float_value = DexMethodId {
        class_descriptor: "Ljava/lang/Number;".to_owned(),
        name: "floatValue".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "F".to_owned(),
            return_type: "F".to_owned(),
            parameters: Vec::new(),
        },
    };
    let float_invocation = test_invocation(vec![DexValue::Float(1.5)]);
    let bits_invocation = test_invocation(vec![DexValue::Int(1.5_f32.to_bits() as i32)]);
    let boxed_value = DexValue::Reference(std::rc::Rc::from(format!(
        "Ljava/lang/Float;:{:08x}",
        1.5_f32.to_bits()
    )));
    let unbox_invocation = test_invocation(vec![boxed_value]);

    assert_eq!(
        program.invoke_builtin_method(&value_of, Some(&float_invocation)),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from(format!("Ljava/lang/Float;:{:08x}", 1.5_f32.to_bits()))
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&value_of, Some(&bits_invocation)),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from(format!("Ljava/lang/Float;:{:08x}", 1.5_f32.to_bits()))
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&float_value, Some(&unbox_invocation)),
        Some(DexExecutionResult::Value(DexValue::Float(1.5)))
    );
    assert_eq!(
        program.invoke_builtin_method(&number_float_value, Some(&unbox_invocation)),
        Some(DexExecutionResult::Value(DexValue::Float(1.5)))
    );
}

#[test]
fn builtin_float_constructor_stores_real_object_value() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Ljava/lang/Float;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VF".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["F".to_owned()],
        },
    };
    let number_float_value = DexMethodId {
        class_descriptor: "Ljava/lang/Number;".to_owned(),
        name: "floatValue".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "F".to_owned(),
            return_type: "F".to_owned(),
            parameters: Vec::new(),
        },
    };
    let object = DexValue::Object(DexObjectRef {
        id: 12,
        class_descriptor: std::rc::Rc::from("Ljava/lang/Float;"),
    });
    let value_field = DexFieldId {
        class_descriptor: "Ljava/lang/Float;".to_owned(),
        type_descriptor: "F".to_owned(),
        name: "value".to_owned(),
    };

    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![
                object.clone(),
                DexValue::Int(1.5_f32.to_bits() as i32)
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&object, &value_field),
        Some(DexValue::Float(1.5))
    );
    assert_eq!(
        program.invoke_builtin_method(&number_float_value, Some(&test_invocation(vec![object]))),
        Some(DexExecutionResult::Value(DexValue::Float(1.5)))
    );
}

#[test]
fn builtin_long_get_long_returns_default_for_missing_property() {
    let program = DexProgram::new(Vec::new());
    let get_long = DexMethodId {
        class_descriptor: "Ljava/lang/Long;".to_owned(),
        name: "getLong".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LLJ".to_owned(),
            return_type: "Ljava/lang/Long;".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned(), "J".to_owned()],
        },
    };
    let invocation = test_invocation(vec![
        DexValue::String(std::rc::Rc::from("winedroid.missing.long")),
        DexValue::Long(1000),
        DexValue::Long(1000),
    ]);

    assert_eq!(
        program.invoke_builtin_method(&get_long, Some(&invocation)),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Ljava/lang/Long;:1000")
        )))
    );
}

#[test]
fn builtin_long_long_value_unboxes_reference() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/lang/Long;".to_owned(),
        name: "longValue".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "J".to_owned(),
            return_type: "J".to_owned(),
            parameters: Vec::new(),
        },
    };
    let invocation = test_invocation(vec![DexValue::Reference(std::rc::Rc::from(
        "Ljava/lang/Long;:1000",
    ))]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Value(DexValue::Long(1000)))
    );
}

#[test]
fn decodes_java_long_property_values() {
    assert_eq!(java_long_decode("42"), Some(42));
    assert_eq!(java_long_decode("0x2a"), Some(42));
    assert_eq!(java_long_decode("#2a"), Some(42));
    assert_eq!(java_long_decode("052"), Some(42));
    assert_eq!(java_long_decode("-42"), Some(-42));
}

#[test]
fn builtin_locale_get_default_returns_bootstrap_locale() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/util/Locale;".to_owned(),
        name: "getDefault".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/util/Locale;".to_owned(),
            parameters: Vec::new(),
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&method, None),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Ljava/util/Locale;:default")
        )))
    );
}

#[test]
fn builtin_locale_for_language_tag_returns_tagged_locale() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/util/Locale;".to_owned(),
        name: "forLanguageTag".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/util/Locale;".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned()],
        },
    };
    let invocation = test_invocation(vec![DexValue::String(std::rc::Rc::from("en-US"))]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Ljava/util/Locale;:en-US")
        )))
    );
}

#[test]
fn builtin_locale_get_country_returns_bootstrap_country() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/util/Locale;".to_owned(),
        name: "getCountry".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let invocation = test_invocation(vec![DexValue::Reference(std::rc::Rc::from(
        "Ljava/util/Locale;:default",
    ))]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("US")
        )))
    );
}

#[test]
fn builtin_locale_get_display_language_returns_bootstrap_language() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/util/Locale;".to_owned(),
        name: "getDisplayLanguage".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let invocation = test_invocation(vec![DexValue::Reference(std::rc::Rc::from(
        "Ljava/util/Locale;:default",
    ))]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("English")
        )))
    );
}

#[test]
fn art_integer_int_value_on_null_receiver_throws_null_pointer_exception() {
    // cite: art common_throws.cc#ThrowNullPointerExceptionForMethodAccess
    // cite: art interpreter_common.h#DoInvoke
    let program = DexProgram::new(Vec::new());

    for class_descriptor in ["Ljava/lang/Integer;", "Ljava/lang/Number;"] {
        let int_value = builtin_method(class_descriptor, "intValue", "I", &[]);

        let exception = thrown_exception(
            program.invoke_builtin_method(&int_value, Some(&test_invocation(vec![DexValue::Null]))),
        );

        assert!(program.value_matches_type(&exception, "Ljava/lang/NullPointerException;"));
        assert_eq!(
            program.invoke_builtin_method(
                &int_value,
                Some(&test_invocation(vec![DexValue::Int(0)]))
            ),
            Some(DexExecutionResult::Int(0))
        );
    }
}

#[test]
fn libcore_long_parse_long_accepts_range_and_throws_number_format_exception_outside_it() {
    // cite: libcore Long.java#parseLong
    // cite: libcore NumberFormatException.java#forInputString
    let program = DexProgram::new(Vec::new());
    let parse_long = builtin_method("Ljava/lang/Long;", "parseLong", "J", &["Ljava/lang/String;"]);
    let parse = |input: DexValue| {
        program.invoke_builtin_method(&parse_long, Some(&test_invocation(vec![input])))
    };
    let string = |value: &str| DexValue::String(std::rc::Rc::from(value));

    for (input, expected) in [
        ("0", 0),
        ("-0", 0),
        ("+5", 5),
        ("9223372036854775807", i64::MAX),
        ("-9223372036854775808", i64::MIN),
    ] {
        assert_eq!(
            parse(string(input)),
            Some(DexExecutionResult::Value(DexValue::Long(expected))),
            "{input}"
        );
    }

    for input in [
        "9223372036854775808",
        "-9223372036854775809",
        "",
        "+",
        "-",
        " 1",
        "1 ",
        "1a",
        "1L",
    ] {
        let exception = thrown_exception(parse(string(input)));
        assert!(program.value_matches_type(&exception, "Ljava/lang/NumberFormatException;"));
        assert!(program.value_matches_type(&exception, "Ljava/lang/IllegalArgumentException;"));
        assert_eq!(
            thrown_detail_message(&program, &exception),
            Some(format!("For input string: \"{input}\"")),
            "{input}"
        );
    }

    let exception = thrown_exception(parse(DexValue::Null));
    assert!(program.value_matches_type(&exception, "Ljava/lang/NumberFormatException;"));
    assert_eq!(
        thrown_detail_message(&program, &exception),
        Some("Cannot parse null string".to_owned())
    );
}
