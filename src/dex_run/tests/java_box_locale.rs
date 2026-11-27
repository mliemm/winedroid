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

// cite: libcore Integer.java#getInteger, decode
#[test]
fn builtin_integer_get_integer_reads_system_property_with_default() {
    let program = DexProgram::new(Vec::new());
    let get_integer = DexMethodId {
        class_descriptor: "Ljava/lang/Integer;".to_owned(),
        name: "getInteger".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LLI".to_owned(),
            return_type: "Ljava/lang/Integer;".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned(), "I".to_owned()],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(
            &get_integer,
            Some(&test_invocation(vec![
                DexValue::String(std::rc::Rc::from("java.version")),
                DexValue::Int(7)
            ]))
        ),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Ljava/lang/Integer;:0")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_integer,
            Some(&test_invocation(vec![
                DexValue::String(std::rc::Rc::from("winedroid.missing")),
                DexValue::Int(7)
            ]))
        ),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Ljava/lang/Integer;:7")
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
            program
                .invoke_builtin_method(&int_value, Some(&test_invocation(vec![DexValue::Int(0)]))),
            Some(DexExecutionResult::Int(0))
        );
    }
}

#[test]
fn libcore_long_parse_long_accepts_range_and_throws_number_format_exception_outside_it() {
    // cite: libcore Long.java#parseLong
    // cite: libcore NumberFormatException.java#forInputString
    let program = DexProgram::new(Vec::new());
    let parse_long = builtin_method(
        "Ljava/lang/Long;",
        "parseLong",
        "J",
        &["Ljava/lang/String;"],
    );
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

fn locale_accessor(name: &str, locale: &str) -> Option<DexExecutionResult> {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/util/Locale;".to_owned(),
        name: name.to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let invocation = test_invocation(vec![DexValue::Reference(std::rc::Rc::from(format!(
        "Ljava/util/Locale;:{locale}"
    )))]);

    program.invoke_builtin_method(&method, Some(&invocation))
}

fn locale_string(name: &str, locale: &str) -> String {
    match locale_accessor(name, locale) {
        Some(DexExecutionResult::Value(DexValue::String(value))) => value.to_string(),
        other => panic!("{name} on {locale}: {other:?}"),
    }
}

#[test]
fn libcore_locale_get_language_reads_the_language_subtag_lowercased() {
    // cite: libcore Locale.java#getLanguage, forLanguageTag
    assert_eq!(locale_string("getLanguage", "default"), "en");
    assert_eq!(locale_string("getLanguage", "ROOT"), "");
    assert_eq!(locale_string("getLanguage", "FRANCE"), "fr");
    assert_eq!(locale_string("getLanguage", "ZH-hant-tw"), "zh");
    assert_eq!(locale_string("getLanguage", "pt-BR"), "pt");
}

#[test]
fn libcore_base_locale_maps_he_yi_id_to_the_legacy_iso_codes() {
    // cite: libcore BaseLocale.java#getInstance
    assert_eq!(locale_string("getLanguage", "he-IL"), "iw");
    assert_eq!(locale_string("getLanguage", "yi"), "ji");
    assert_eq!(locale_string("getLanguage", "id-ID"), "in");
}

#[test]
fn libcore_locale_get_country_reads_the_region_subtag_uppercased() {
    // cite: libcore Locale.java#getCountry
    // cite: libcore LanguageTag.java#isRegion
    assert_eq!(locale_string("getCountry", "default"), "US");
    assert_eq!(locale_string("getCountry", "UK"), "GB");
    assert_eq!(locale_string("getCountry", "ENGLISH"), "");
    assert_eq!(locale_string("getCountry", "pt-br"), "BR");
    assert_eq!(locale_string("getCountry", "zh-Hans-CN"), "CN");
    assert_eq!(locale_string("getCountry", "es-419"), "419");
    assert_eq!(locale_string("getCountry", "en-12"), "");
}

#[test]
fn libcore_language_tag_extlang_replaces_the_language_and_und_is_empty() {
    // cite: libcore InternalLocaleBuilder.java#setLanguageTag
    assert_eq!(locale_string("getLanguage", "zh-yue-HK"), "yue");
    assert_eq!(locale_string("getLanguage", "und-US"), "");
    assert_eq!(locale_string("getLanguage", "UND-US"), "und");
    assert_eq!(locale_string("getCountry", "und-US"), "US");
}

#[test]
fn libcore_locale_get_variant_joins_variant_subtags_with_underscore() {
    // cite: libcore InternalLocaleBuilder.java#setLanguageTag
    assert_eq!(locale_string("getVariant", "sl-rozaj-biske"), "rozaj_biske");
    assert_eq!(locale_string("getVariant", "de-DE-1996"), "1996");
    assert_eq!(locale_string("getVariant", "en-US"), "");
}

#[test]
fn libcore_locale_ill_formed_tag_yields_the_root_locale() {
    // cite: libcore LanguageTag.java#parse
    assert_eq!(locale_string("getLanguage", "en_US"), "");
    assert_eq!(locale_string("getLanguage", "x"), "");
    assert_eq!(locale_string("getCountry", "x-US"), "");
}

#[test]
fn libcore_locale_legacy_language_tags_are_a_missing_primitive() {
    // cite: libcore LanguageTag.java#LEGACY
    assert_eq!(locale_accessor("getLanguage", "i-klingon"), None);
    assert_eq!(locale_accessor("getLanguage", "en-GB-oed"), None);
}

fn constructed_locale(
    program: &DexProgram,
    arguments: &[DexValue],
) -> (DexValue, Option<DexExecutionResult>) {
    let locale = program.allocate_object("Ljava/util/Locale;".to_owned());
    let descriptor = if arguments.len() == 2 {
        "(Ljava/lang/String;Ljava/lang/String;)V"
    } else {
        "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)V"
    };
    let constructor = DexMethodId {
        class_descriptor: "Ljava/util/Locale;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: descriptor
                .trim_start_matches('(')
                .trim_end_matches(")V")
                .split_inclusive(';')
                .map(str::to_owned)
                .collect(),
        },
    };
    let mut values = vec![locale.clone()];
    values.extend_from_slice(arguments);
    let result = program.invoke_builtin_method(&constructor, Some(&test_invocation(values)));

    (locale, result)
}

fn string_argument(value: &str) -> DexValue {
    DexValue::String(std::rc::Rc::from(value))
}

fn accessor_on(program: &DexProgram, name: &str, locale: &DexValue) -> Option<DexExecutionResult> {
    let method = DexMethodId {
        class_descriptor: "Ljava/util/Locale;".to_owned(),
        name: name.to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: Vec::new(),
        },
    };

    program.invoke_builtin_method(&method, Some(&test_invocation(vec![locale.clone()])))
}

fn accessor_string(program: &DexProgram, name: &str, locale: &DexValue) -> String {
    match accessor_on(program, name, locale) {
        Some(DexExecutionResult::Value(DexValue::String(value))) => value.to_string(),
        other => panic!("{name}: {other:?}"),
    }
}

#[test]
fn libcore_locale_language_country_constructor_normalizes_case() {
    // cite: libcore Locale.java#Locale
    // cite: libcore BaseLocale.java#getInstance
    let program = DexProgram::new(Vec::new());
    let (locale, result) =
        constructed_locale(&program, &[string_argument("EN"), string_argument("us")]);

    assert_eq!(result, Some(DexExecutionResult::Void));
    assert_eq!(accessor_string(&program, "getLanguage", &locale), "en");
    assert_eq!(accessor_string(&program, "getCountry", &locale), "US");
    assert_eq!(accessor_string(&program, "getVariant", &locale), "");
}

#[test]
fn libcore_locale_constructor_stores_old_iso_language_codes_and_the_variant_as_given() {
    // cite: libcore Locale.java#convertOldISOCodes
    let program = DexProgram::new(Vec::new());
    let (hebrew, _) = constructed_locale(&program, &[string_argument("he"), string_argument("il")]);
    let (indonesian, _) = constructed_locale(
        &program,
        &[
            string_argument("ID"),
            string_argument(""),
            string_argument("Posix"),
        ],
    );

    assert_eq!(accessor_string(&program, "getLanguage", &hebrew), "iw");
    assert_eq!(accessor_string(&program, "getLanguage", &indonesian), "in");
    assert_eq!(
        accessor_string(&program, "getVariant", &indonesian),
        "Posix"
    );
}

#[test]
fn libcore_locale_constructor_null_argument_throws_null_pointer_exception() {
    // cite: libcore Locale.java#Locale
    let program = DexProgram::new(Vec::new());
    let (_, result) = constructed_locale(&program, &[string_argument("en"), DexValue::Null]);

    let Some(DexExecutionResult::Throw(exception)) = result else {
        panic!("null country should throw: {result:?}");
    };
    assert!(program.value_matches_type(&exception, "Ljava/lang/NullPointerException;"));
}

#[test]
fn libcore_locale_three_argument_constructor_null_language_or_variant_throws_null_pointer_exception(
) {
    // cite: libcore Locale.java#Locale
    let program = DexProgram::new(Vec::new());
    for arguments in [
        [DexValue::Null, string_argument("US"), string_argument("")],
        [string_argument("en"), string_argument("US"), DexValue::Null],
    ] {
        let (_, result) = constructed_locale(&program, &arguments);

        let Some(DexExecutionResult::Throw(exception)) = result else {
            panic!("null argument should throw: {result:?}");
        };
        assert!(program.value_matches_type(&exception, "Ljava/lang/NullPointerException;"));
    }
}

#[test]
fn libcore_long_to_hex_string_formats_unsigned_hex() {
    // cite: libcore Long.java#toHexString
    let program = DexProgram::new(Vec::new());
    let method = builtin_method(
        "Ljava/lang/Long;",
        "toHexString",
        "Ljava/lang/String;",
        &["J"],
    );

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::Long(255),
                DexValue::Long(255)
            ]))
        ),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("ff")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::Long(-1),
                DexValue::Long(-1)
            ]))
        ),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("ffffffffffffffff")
        )))
    );
}
