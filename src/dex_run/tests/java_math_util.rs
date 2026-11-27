use super::super::*;
use super::helpers::*;

// cite: libcore Math.java#random, RandomNumberGeneratorHolder
// cite: libcore Random.java#nextDouble
#[test]
fn math_random_reuses_its_generator_across_calls() {
    let program = DexProgram::new(Vec::new());
    program.runtime.borrow_mut().boot_wall_clock_millis = 0;
    let method = builtin_method("Ljava/lang/Math;", "random", "D", &[]);
    for expected in [0.730967787376657_f64, 0.24053641567148587] {
        assert_eq!(
            program.invoke_builtin_method(&method, None),
            Some(DexExecutionResult::Value(DexValue::Long(
                expected.to_bits() as i64
            ))),
        );
    }
}

#[test]
fn builtin_uuid_random_uuid_returns_version_four_variant_ietf() {
    let program = DexProgram::new(Vec::new());
    let random_uuid = DexMethodId {
        class_descriptor: "Ljava/util/UUID;".to_owned(),
        name: "randomUUID".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/util/UUID;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let to_string = DexMethodId {
        class_descriptor: "Ljava/util/UUID;".to_owned(),
        name: "toString".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: Vec::new(),
        },
    };
    // cite: libcore UUID.java#randomUUID
    let Some(DexExecutionResult::Value(first)) = program.invoke_builtin_method(&random_uuid, None)
    else {
        panic!("UUID.randomUUID should return a UUID value");
    };
    let Some(DexExecutionResult::Value(second)) = program.invoke_builtin_method(&random_uuid, None)
    else {
        panic!("UUID.randomUUID should return a UUID value");
    };
    assert_ne!(first, second);

    let Some(DexExecutionResult::Value(DexValue::String(text))) =
        program.invoke_builtin_method(&to_string, Some(&test_invocation(vec![first.clone()])))
    else {
        panic!("UUID.toString should return a string");
    };
    assert_eq!(text.len(), 36);
    assert_eq!(text.as_bytes()[14], b'4');
    assert!(matches!(text.as_bytes()[19], b'8' | b'9' | b'a' | b'b'));
    let DexValue::Reference(reference) = first else {
        panic!("UUID.randomUUID should return a reference");
    };
    assert_eq!(
        reference,
        std::rc::Rc::from(format!("Ljava/util/UUID;:{text}"))
    );
}

#[test]
fn builtin_big_decimal_zero_to_string_returns_zero() {
    let program = DexProgram::new(Vec::new());
    let zero_field = DexFieldId {
        class_descriptor: "Ljava/math/BigDecimal;".to_owned(),
        type_descriptor: "Ljava/math/BigDecimal;".to_owned(),
        name: "ZERO".to_owned(),
    };
    let one_field = DexFieldId {
        class_descriptor: "Ljava/math/BigDecimal;".to_owned(),
        type_descriptor: "Ljava/math/BigDecimal;".to_owned(),
        name: "ONE".to_owned(),
    };
    let ten_field = DexFieldId {
        class_descriptor: "Ljava/math/BigDecimal;".to_owned(),
        type_descriptor: "Ljava/math/BigDecimal;".to_owned(),
        name: "TEN".to_owned(),
    };
    let to_string = DexMethodId {
        class_descriptor: "Ljava/math/BigDecimal;".to_owned(),
        name: "toString".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let float_value = DexMethodId {
        class_descriptor: "Ljava/math/BigDecimal;".to_owned(),
        name: "floatValue".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "F".to_owned(),
            return_type: "F".to_owned(),
            parameters: Vec::new(),
        },
    };
    let double_value = DexMethodId {
        class_descriptor: "Ljava/math/BigDecimal;".to_owned(),
        name: "doubleValue".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "D".to_owned(),
            return_type: "D".to_owned(),
            parameters: Vec::new(),
        },
    };
    let int_value = DexMethodId {
        class_descriptor: "Ljava/math/BigDecimal;".to_owned(),
        name: "intValue".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let value_of_double = DexMethodId {
        class_descriptor: "Ljava/math/BigDecimal;".to_owned(),
        name: "valueOf".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LD".to_owned(),
            return_type: "Ljava/math/BigDecimal;".to_owned(),
            parameters: vec!["D".to_owned()],
        },
    };
    let constructor = DexMethodId {
        class_descriptor: "Ljava/math/BigDecimal;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned()],
        },
    };
    let int_constructor = DexMethodId {
        class_descriptor: "Ljava/math/BigDecimal;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let double_constructor = DexMethodId {
        class_descriptor: "Ljava/math/BigDecimal;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VD".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["D".to_owned()],
        },
    };
    let big_decimal_big_integer_constructor = DexMethodId {
        class_descriptor: "Ljava/math/BigDecimal;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/math/BigInteger;".to_owned()],
        },
    };
    let big_integer_constructor = DexMethodId {
        class_descriptor: "Ljava/math/BigInteger;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned()],
        },
    };
    let big_integer_to_string = DexMethodId {
        class_descriptor: "Ljava/math/BigInteger;".to_owned(),
        name: "toString".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let big_integer_value_of = DexMethodId {
        class_descriptor: "Ljava/math/BigInteger;".to_owned(),
        name: "valueOf".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LJ".to_owned(),
            return_type: "Ljava/math/BigInteger;".to_owned(),
            parameters: vec!["J".to_owned()],
        },
    };
    let big_integer_multiply = DexMethodId {
        class_descriptor: "Ljava/math/BigInteger;".to_owned(),
        name: "multiply".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/math/BigInteger;".to_owned(),
            parameters: vec!["Ljava/math/BigInteger;".to_owned()],
        },
    };
    let set_scale = DexMethodId {
        class_descriptor: "Ljava/math/BigDecimal;".to_owned(),
        name: "setScale".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LIL".to_owned(),
            return_type: "Ljava/math/BigDecimal;".to_owned(),
            parameters: vec!["I".to_owned(), "Ljava/math/RoundingMode;".to_owned()],
        },
    };
    let add = DexMethodId {
        class_descriptor: "Ljava/math/BigDecimal;".to_owned(),
        name: "add".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/math/BigDecimal;".to_owned(),
            parameters: vec!["Ljava/math/BigDecimal;".to_owned()],
        },
    };
    let subtract = DexMethodId {
        class_descriptor: "Ljava/math/BigDecimal;".to_owned(),
        name: "subtract".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/math/BigDecimal;".to_owned(),
            parameters: vec!["Ljava/math/BigDecimal;".to_owned()],
        },
    };
    let multiply = DexMethodId {
        class_descriptor: "Ljava/math/BigDecimal;".to_owned(),
        name: "multiply".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/math/BigDecimal;".to_owned(),
            parameters: vec!["Ljava/math/BigDecimal;".to_owned()],
        },
    };
    let divide = DexMethodId {
        class_descriptor: "Ljava/math/BigDecimal;".to_owned(),
        name: "divide".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/math/BigDecimal;".to_owned(),
            parameters: vec!["Ljava/math/BigDecimal;".to_owned()],
        },
    };
    let sqrt = DexMethodId {
        class_descriptor: "Ljava/math/BigDecimal;".to_owned(),
        name: "sqrt".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/math/BigDecimal;".to_owned(),
            parameters: vec!["Ljava/math/MathContext;".to_owned()],
        },
    };
    let pow = DexMethodId {
        class_descriptor: "Ljava/math/BigDecimal;".to_owned(),
        name: "pow".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LIL".to_owned(),
            return_type: "Ljava/math/BigDecimal;".to_owned(),
            parameters: vec!["I".to_owned(), "Ljava/math/MathContext;".to_owned()],
        },
    };
    let compare_to = DexMethodId {
        class_descriptor: "Ljava/math/BigDecimal;".to_owned(),
        name: "compareTo".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "IL".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["Ljava/math/BigDecimal;".to_owned()],
        },
    };
    let math_context_constructor = DexMethodId {
        class_descriptor: "Ljava/math/MathContext;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VIL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned(), "Ljava/math/RoundingMode;".to_owned()],
        },
    };
    let zero = DexValue::Reference(std::rc::Rc::from("Ljava/math/BigDecimal;:0"));
    let seven = DexValue::Object(DexObjectRef {
        id: 7,
        class_descriptor: std::rc::Rc::from("Ljava/math/BigDecimal;"),
    });
    let seven_half = DexValue::Object(DexObjectRef {
        id: 8,
        class_descriptor: std::rc::Rc::from("Ljava/math/BigDecimal;"),
    });
    let negative_two_quarter = DexValue::Object(DexObjectRef {
        id: 9,
        class_descriptor: std::rc::Rc::from("Ljava/math/BigDecimal;"),
    });
    let one_point_two = DexValue::Object(DexObjectRef {
        id: 10,
        class_descriptor: std::rc::Rc::from("Ljava/math/BigDecimal;"),
    });
    let three_point_four_five = DexValue::Object(DexObjectRef {
        id: 11,
        class_descriptor: std::rc::Rc::from("Ljava/math/BigDecimal;"),
    });
    let multiply_left = DexValue::Object(DexObjectRef {
        id: 12,
        class_descriptor: std::rc::Rc::from("Ljava/math/BigDecimal;"),
    });
    let multiply_right = DexValue::Object(DexObjectRef {
        id: 13,
        class_descriptor: std::rc::Rc::from("Ljava/math/BigDecimal;"),
    });
    let subtract_left = DexValue::Object(DexObjectRef {
        id: 14,
        class_descriptor: std::rc::Rc::from("Ljava/math/BigDecimal;"),
    });
    let subtract_right = DexValue::Object(DexObjectRef {
        id: 15,
        class_descriptor: std::rc::Rc::from("Ljava/math/BigDecimal;"),
    });
    let divide_left = DexValue::Object(DexObjectRef {
        id: 16,
        class_descriptor: std::rc::Rc::from("Ljava/math/BigDecimal;"),
    });
    let divide_right = DexValue::Object(DexObjectRef {
        id: 17,
        class_descriptor: std::rc::Rc::from("Ljava/math/BigDecimal;"),
    });
    let one = DexValue::Object(DexObjectRef {
        id: 18,
        class_descriptor: std::rc::Rc::from("Ljava/math/BigDecimal;"),
    });
    let three = DexValue::Object(DexObjectRef {
        id: 19,
        class_descriptor: std::rc::Rc::from("Ljava/math/BigDecimal;"),
    });
    let math_context = DexValue::Object(DexObjectRef {
        id: 20,
        class_descriptor: std::rc::Rc::from("Ljava/math/MathContext;"),
    });
    let square_root_nine = DexValue::Object(DexObjectRef {
        id: 21,
        class_descriptor: std::rc::Rc::from("Ljava/math/BigDecimal;"),
    });
    let square_root_two = DexValue::Object(DexObjectRef {
        id: 22,
        class_descriptor: std::rc::Rc::from("Ljava/math/BigDecimal;"),
    });
    let int_constructed = DexValue::Object(DexObjectRef {
        id: 23,
        class_descriptor: std::rc::Rc::from("Ljava/math/BigDecimal;"),
    });
    let double_constructed = DexValue::Object(DexObjectRef {
        id: 24,
        class_descriptor: std::rc::Rc::from("Ljava/math/BigDecimal;"),
    });
    let int_value_fraction = DexValue::Object(DexObjectRef {
        id: 25,
        class_descriptor: std::rc::Rc::from("Ljava/math/BigDecimal;"),
    });
    let int_value_overflow = DexValue::Object(DexObjectRef {
        id: 26,
        class_descriptor: std::rc::Rc::from("Ljava/math/BigDecimal;"),
    });
    let power_base = DexValue::Object(DexObjectRef {
        id: 27,
        class_descriptor: std::rc::Rc::from("Ljava/math/BigDecimal;"),
    });
    let big_integer = DexValue::Object(DexObjectRef {
        id: 28,
        class_descriptor: std::rc::Rc::from("Ljava/math/BigInteger;"),
    });
    let big_decimal_from_big_integer = DexValue::Object(DexObjectRef {
        id: 29,
        class_descriptor: std::rc::Rc::from("Ljava/math/BigDecimal;"),
    });
    let rounded_scale = DexValue::Object(DexObjectRef {
        id: 30,
        class_descriptor: std::rc::Rc::from("Ljava/math/BigDecimal;"),
    });
    let half_even = DexValue::Reference(std::rc::Rc::from("Ljava/math/RoundingMode;.HALF_EVEN"));
    let half_even_field = DexFieldId {
        class_descriptor: "Ljava/math/RoundingMode;".to_owned(),
        type_descriptor: "Ljava/math/RoundingMode;".to_owned(),
        name: "HALF_EVEN".to_owned(),
    };
    let decimal64_field = DexFieldId {
        class_descriptor: "Ljava/math/MathContext;".to_owned(),
        type_descriptor: "Ljava/math/MathContext;".to_owned(),
        name: "DECIMAL64".to_owned(),
    };

    assert_eq!(bootstrap_static_field(&zero_field), Some(zero.clone()));
    assert_eq!(
        bootstrap_static_field(&one_field),
        Some(DexValue::Reference(std::rc::Rc::from(
            "Ljava/math/BigDecimal;:1"
        )))
    );
    assert_eq!(
        bootstrap_static_field(&ten_field),
        Some(DexValue::Reference(std::rc::Rc::from(
            "Ljava/math/BigDecimal;:10"
        )))
    );
    assert_eq!(
        bootstrap_static_field(&half_even_field),
        Some(half_even.clone())
    );
    assert_eq!(
        bootstrap_static_field(&decimal64_field),
        Some(DexValue::Reference(std::rc::Rc::from(
            "Ljava/math/MathContext;.DECIMAL64"
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&to_string, Some(&test_invocation(vec![zero]))),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("0")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &math_context_constructor,
            Some(&test_invocation(vec![
                math_context.clone(),
                DexValue::Int(34),
                half_even.clone(),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .math_context_values
            .get("object:20")
            .cloned(),
        Some(MathContextRuntimeValue {
            precision: 34,
            rounding_mode: "HALF_EVEN".to_owned(),
        })
    );
    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![
                seven.clone(),
                DexValue::String(std::rc::Rc::from("7"))
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&to_string, Some(&test_invocation(vec![seven]))),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("7")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &int_constructor,
            Some(&test_invocation(vec![
                int_constructed.clone(),
                DexValue::Int(-42)
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&to_string, Some(&test_invocation(vec![int_constructed]))),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("-42")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &double_constructor,
            Some(&test_invocation(vec![
                double_constructed.clone(),
                DexValue::Long(0.1_f64.to_bits() as i64),
                DexValue::Long(0.1_f64.to_bits() as i64),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&to_string, Some(&test_invocation(vec![double_constructed]))),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("0.1000000000000000055511151231257827021181583404541015625")
        )))
    );
    let Some(DexExecutionResult::Value(value_of_decimal)) = program.invoke_builtin_method(
        &value_of_double,
        Some(&test_invocation(vec![
            DexValue::Long(8.0_f64.to_bits() as i64),
            DexValue::Long(8.0_f64.to_bits() as i64),
        ])),
    ) else {
        panic!("BigDecimal.valueOf should return a BigDecimal");
    };
    assert_eq!(
        program.invoke_builtin_method(&to_string, Some(&test_invocation(vec![value_of_decimal]))),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("8.0")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &big_integer_constructor,
            Some(&test_invocation(vec![
                big_integer.clone(),
                DexValue::String(std::rc::Rc::from("+000120"))
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &big_integer_to_string,
            Some(&test_invocation(vec![big_integer.clone()]))
        ),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("120")
        )))
    );
    let Some(DexExecutionResult::Value(value_of_big_integer)) = program.invoke_builtin_method(
        &big_integer_value_of,
        Some(&test_invocation(vec![
            DexValue::Long((-7_i64) as i64),
            DexValue::Long((-7_i64) as i64),
        ])),
    ) else {
        panic!("BigInteger.valueOf should return a BigInteger");
    };
    assert_eq!(
        program.invoke_builtin_method(
            &big_integer_to_string,
            Some(&test_invocation(vec![value_of_big_integer]))
        ),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("-7")
        )))
    );
    let Some(DexExecutionResult::Value(value_of_ten)) = program.invoke_builtin_method(
        &big_integer_value_of,
        Some(&test_invocation(vec![
            DexValue::Long(10),
            DexValue::Long(10),
        ])),
    ) else {
        panic!("BigInteger.valueOf should return a BigInteger");
    };
    let Some(DexExecutionResult::Value(product)) = program.invoke_builtin_method(
        &big_integer_multiply,
        Some(&test_invocation(vec![big_integer.clone(), value_of_ten])),
    ) else {
        panic!("BigInteger.multiply should return a BigInteger");
    };
    assert_eq!(
        program.invoke_builtin_method(
            &big_integer_to_string,
            Some(&test_invocation(vec![product]))
        ),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("1200")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &big_decimal_big_integer_constructor,
            Some(&test_invocation(vec![
                big_decimal_from_big_integer.clone(),
                big_integer
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &to_string,
            Some(&test_invocation(vec![big_decimal_from_big_integer]))
        ),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("120")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![
                rounded_scale.clone(),
                DexValue::String(std::rc::Rc::from("2.675"))
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    let Some(DexExecutionResult::Value(rounded)) = program.invoke_builtin_method(
        &set_scale,
        Some(&test_invocation(vec![
            rounded_scale,
            DexValue::Int(2),
            half_even.clone(),
        ])),
    ) else {
        panic!("setScale should round and return a BigDecimal");
    };
    assert_eq!(
        program.invoke_builtin_method(&to_string, Some(&test_invocation(vec![rounded]))),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("2.68")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &float_value,
            Some(&test_invocation(vec![DexValue::Object(DexObjectRef {
                id: 7,
                class_descriptor: std::rc::Rc::from("Ljava/math/BigDecimal;"),
            })]))
        ),
        Some(DexExecutionResult::Value(DexValue::Float(7.0)))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &double_value,
            Some(&test_invocation(vec![DexValue::Object(DexObjectRef {
                id: 7,
                class_descriptor: std::rc::Rc::from("Ljava/math/BigDecimal;"),
            })]))
        ),
        Some(DexExecutionResult::Value(DexValue::Long(
            7.0_f64.to_bits() as i64
        )))
    );
    let Some(DexExecutionResult::Value(scaled)) = program.invoke_builtin_method(
        &set_scale,
        Some(&test_invocation(vec![
            DexValue::Object(DexObjectRef {
                id: 7,
                class_descriptor: std::rc::Rc::from("Ljava/math/BigDecimal;"),
            }),
            DexValue::Int(3),
            half_even,
        ])),
    ) else {
        panic!("setScale should return a BigDecimal");
    };
    assert_eq!(
        program.invoke_builtin_method(&to_string, Some(&test_invocation(vec![scaled]))),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("7.000")
        )))
    );
    let Some(DexExecutionResult::Value(sum)) = program.invoke_builtin_method(
        &add,
        Some(&test_invocation(vec![
            DexValue::Object(DexObjectRef {
                id: 7,
                class_descriptor: std::rc::Rc::from("Ljava/math/BigDecimal;"),
            }),
            DexValue::Reference(std::rc::Rc::from("Ljava/math/BigDecimal;:0")),
        ])),
    ) else {
        panic!("add should return a BigDecimal");
    };
    assert_eq!(
        program.invoke_builtin_method(&to_string, Some(&test_invocation(vec![sum]))),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("7")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![
                seven_half.clone(),
                DexValue::String(std::rc::Rc::from("7.50"))
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![
                negative_two_quarter.clone(),
                DexValue::String(std::rc::Rc::from("-2.25"))
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    let Some(DexExecutionResult::Value(sum)) = program.invoke_builtin_method(
        &add,
        Some(&test_invocation(vec![seven_half, negative_two_quarter])),
    ) else {
        panic!("add should return a BigDecimal");
    };
    assert_eq!(
        program.invoke_builtin_method(&to_string, Some(&test_invocation(vec![sum]))),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("5.25")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![
                one_point_two.clone(),
                DexValue::String(std::rc::Rc::from("1.2"))
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![
                three_point_four_five.clone(),
                DexValue::String(std::rc::Rc::from("3.45"))
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    let Some(DexExecutionResult::Value(sum)) = program.invoke_builtin_method(
        &add,
        Some(&test_invocation(vec![one_point_two, three_point_four_five])),
    ) else {
        panic!("add should return a BigDecimal");
    };
    assert_eq!(
        program.invoke_builtin_method(&to_string, Some(&test_invocation(vec![sum]))),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("4.65")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![
                multiply_left.clone(),
                DexValue::String(std::rc::Rc::from("7.50"))
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![
                multiply_right.clone(),
                DexValue::String(std::rc::Rc::from("-2.25"))
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    let Some(DexExecutionResult::Value(product)) = program.invoke_builtin_method(
        &multiply,
        Some(&test_invocation(vec![multiply_left, multiply_right])),
    ) else {
        panic!("multiply should return a BigDecimal");
    };
    assert_eq!(
        program.invoke_builtin_method(&to_string, Some(&test_invocation(vec![product]))),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("-16.8750")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![
                subtract_left.clone(),
                DexValue::String(std::rc::Rc::from("7.50"))
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![
                subtract_right.clone(),
                DexValue::String(std::rc::Rc::from("2.25"))
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    let Some(DexExecutionResult::Value(difference)) = program.invoke_builtin_method(
        &subtract,
        Some(&test_invocation(vec![subtract_left, subtract_right])),
    ) else {
        panic!("subtract should return a BigDecimal");
    };
    assert_eq!(
        program.invoke_builtin_method(&to_string, Some(&test_invocation(vec![difference]))),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("5.25")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![
                divide_left.clone(),
                DexValue::String(std::rc::Rc::from("8"))
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![
                divide_right.clone(),
                DexValue::String(std::rc::Rc::from("4"))
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    let Some(DexExecutionResult::Value(quotient)) = program.invoke_builtin_method(
        &divide,
        Some(&test_invocation(vec![divide_left, divide_right])),
    ) else {
        panic!("divide should return a BigDecimal");
    };
    assert_eq!(
        program.invoke_builtin_method(&to_string, Some(&test_invocation(vec![quotient]))),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("2")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![
                one.clone(),
                DexValue::String(std::rc::Rc::from("1"))
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![
                three.clone(),
                DexValue::String(std::rc::Rc::from("3"))
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&divide, Some(&test_invocation(vec![one, three]))),
        None
    );
    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![
                square_root_nine.clone(),
                DexValue::String(std::rc::Rc::from("9"))
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    let Some(DexExecutionResult::Value(root)) = program.invoke_builtin_method(
        &sqrt,
        Some(&test_invocation(vec![
            square_root_nine,
            math_context.clone(),
        ])),
    ) else {
        panic!("sqrt should return a BigDecimal");
    };
    assert_eq!(
        program.invoke_builtin_method(&to_string, Some(&test_invocation(vec![root]))),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("3")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![
                square_root_two.clone(),
                DexValue::String(std::rc::Rc::from("2"))
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    let Some(DexExecutionResult::Value(root)) = program.invoke_builtin_method(
        &sqrt,
        Some(&test_invocation(vec![
            square_root_two,
            math_context.clone(),
        ])),
    ) else {
        panic!("sqrt should return a BigDecimal");
    };
    assert_eq!(
        program.invoke_builtin_method(&to_string, Some(&test_invocation(vec![root]))),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("1.414213562373095048801688724209698")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![
                power_base.clone(),
                DexValue::String(std::rc::Rc::from("2"))
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    let Some(DexExecutionResult::Value(power)) = program.invoke_builtin_method(
        &pow,
        Some(&test_invocation(vec![
            power_base,
            DexValue::Int(3),
            math_context,
        ])),
    ) else {
        panic!("pow should return a BigDecimal");
    };
    assert_eq!(
        program.invoke_builtin_method(&to_string, Some(&test_invocation(vec![power]))),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("8")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &compare_to,
            Some(&test_invocation(vec![
                DexValue::Reference(std::rc::Rc::from("Ljava/math/BigDecimal;:7.5")),
                DexValue::Reference(std::rc::Rc::from("Ljava/math/BigDecimal;:7.50")),
            ]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &compare_to,
            Some(&test_invocation(vec![
                DexValue::Reference(std::rc::Rc::from("Ljava/math/BigDecimal;:-2.25")),
                DexValue::Reference(std::rc::Rc::from("Ljava/math/BigDecimal;:0")),
            ]))
        ),
        Some(DexExecutionResult::Int(-1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &compare_to,
            Some(&test_invocation(vec![
                DexValue::Reference(std::rc::Rc::from("Ljava/math/BigDecimal;:8")),
                DexValue::Reference(std::rc::Rc::from("Ljava/math/BigDecimal;:7.999")),
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![
                int_value_fraction.clone(),
                DexValue::String(std::rc::Rc::from("-123.99"))
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&int_value, Some(&test_invocation(vec![int_value_fraction]))),
        Some(DexExecutionResult::Int(-123))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![
                int_value_overflow.clone(),
                DexValue::String(std::rc::Rc::from("2147483648"))
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&int_value, Some(&test_invocation(vec![int_value_overflow]))),
        Some(DexExecutionResult::Int(i32::MIN))
    );
}

// cite: libcore BigDecimal.java#setScale, ROUND_HALF_UP, ROUND_DOWN
#[test]
fn libcore_big_decimal_legacy_set_scale_rounds_with_int_modes() {
    let program = DexProgram::new(Vec::new());
    let value = program.allocate_object("Ljava/math/BigDecimal;".to_owned());
    program
        .runtime
        .borrow_mut()
        .big_decimal_values
        .insert(map_identity_key(&value), "2.345".to_owned());
    let set_scale = builtin_method(
        "Ljava/math/BigDecimal;",
        "setScale",
        "Ljava/math/BigDecimal;",
        &["I", "I"],
    );
    let scaled = |mode: i32| match program.invoke_builtin_method(
        &set_scale,
        Some(&test_invocation(vec![
            value.clone(),
            DexValue::Int(2),
            DexValue::Int(mode),
        ])),
    ) {
        Some(DexExecutionResult::Value(result)) => program
            .runtime
            .borrow()
            .big_decimal_values
            .get(&map_identity_key(&result))
            .cloned(),
        other => panic!("setScale should return a BigDecimal, got {other:?}"),
    };

    assert_eq!(scaled(4).as_deref(), Some("2.35"));
    assert_eq!(scaled(1).as_deref(), Some("2.34"));
    assert!(matches!(
        program.invoke_builtin_method(
            &set_scale,
            Some(&test_invocation(vec![
                value,
                DexValue::Int(2),
                DexValue::Int(9)
            ]))
        ),
        Some(DexExecutionResult::Throw(_))
    ));
}

// cite: libcore Integer.java#toString, MIN_VALUE
#[test]
fn libcore_integer_to_string_formats_signed_decimal() {
    let program = DexProgram::new(Vec::new());
    let to_string = builtin_method(
        "Ljava/lang/Integer;",
        "toString",
        "Ljava/lang/String;",
        &["I"],
    );
    for (value, expected) in [
        (0, "0"),
        (-42, "-42"),
        (i32::MIN, "-2147483648"),
        (i32::MAX, "2147483647"),
    ] {
        assert_eq!(
            program.invoke_builtin_method(
                &to_string,
                Some(&test_invocation(vec![DexValue::Int(value)]))
            ),
            Some(DexExecutionResult::Value(DexValue::String(expected.into())))
        );
    }
}

// cite: libcore FloatingDecimal.java#readJavaFormatString
#[test]
fn libcore_double_parse_double_accepts_only_java_number_syntax() {
    let program = DexProgram::new(Vec::new());
    let parse = builtin_method(
        "Ljava/lang/Double;",
        "parseDouble",
        "D",
        &["Ljava/lang/String;"],
    );
    let parsed = |text: &str| match program.invoke_builtin_method(
        &parse,
        Some(&test_invocation(vec![DexValue::String(text.into())])),
    ) {
        Some(DexExecutionResult::Value(DexValue::Long(bits))) => Some(f64::from_bits(bits as u64)),
        _ => None,
    };

    for (text, expected) in [
        (" 12.5 ", 12.5),
        ("-1e3", -1000.0),
        ("+.5", 0.5),
        ("7.", 7.0),
        ("2.5f", 2.5),
        ("3D", 3.0),
        ("\t\n1\u{1}", 1.0),
    ] {
        assert_eq!(parsed(text), Some(expected), "{text:?}");
    }
    assert_eq!(parsed("-Infinity"), Some(f64::NEG_INFINITY));
    assert!(parsed("NaN").unwrap().is_nan());
    for bad in [
        "inf",
        "infinity",
        "nan",
        "5dd",
        "",
        ".",
        "1e",
        "1e+",
        "\u{2003}1",
        "0x10",
    ] {
        assert_eq!(parsed(bad), None, "{bad:?}");
        let Some(DexExecutionResult::Throw(exception)) = program.invoke_builtin_method(
            &parse,
            Some(&test_invocation(vec![DexValue::String(bad.into())])),
        ) else {
            panic!("{bad:?} should throw");
        };
        assert!(program.value_matches_type(&exception, "Ljava/lang/NumberFormatException;"));
    }
}

// cite: libcore Double.java#toString
// cite: libcore Float.java#toString
#[test]
fn libcore_floating_to_string_switches_to_scientific_outside_the_plain_range() {
    for (value, expected) in [
        (0.5f64, "0.5"),
        (100.0, "100.0"),
        (1234567.0, "1234567.0"),
        (1.0e7, "1.0E7"),
        (12345678.9, "1.23456789E7"),
        (0.001, "0.001"),
        (0.0001, "1.0E-4"),
        (-0.0, "-0.0"),
        (-2.5e-10, "-2.5E-10"),
        (0.1 + 0.2, "0.30000000000000004"),
    ] {
        assert_eq!(
            crate::dex_run::java_big_decimal::java_double_to_string(value).as_deref(),
            Some(expected),
            "{value}"
        );
    }
    for (value, expected) in [
        (1.0e10f32, "1.0E10"),
        (3.25f32, "3.25"),
        (1.0e-5f32, "1.0E-5"),
    ] {
        assert_eq!(
            java_float_to_string(value).as_deref(),
            Some(expected),
            "{value}"
        );
    }
}
