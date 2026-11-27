use super::super::*;
use super::helpers::*;

fn graphics_method(
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
            parameters: parameters
                .iter()
                .map(|parameter| (*parameter).to_owned())
                .collect(),
        },
    }
}

#[test]
fn cts_font_variation_axis_construction_and_getters() {
    // cite: cts FontVariationAxisTest.java#testConstruction, testGetterTest, testInvalidTagConstructionTest
    let program = DexProgram::new(Vec::new());
    let class = "Landroid/graphics/fonts/FontVariationAxis;";
    let constructor = graphics_method(class, "<init>", "V", &["Ljava/lang/String;", "F"]);
    let get_tag = graphics_method(class, "getTag", "Ljava/lang/String;", &[]);
    let get_style = graphics_method(class, "getStyleValue", "F", &[]);
    let get_tag_value = graphics_method(class, "getOpenTypeTagValue", "I", &[]);
    let axis = program.allocate_object(class.to_owned());
    let tag = DexValue::String(std::rc::Rc::from("wght"));
    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![
                axis.clone(),
                tag.clone(),
                DexValue::Float(1.0)
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&get_tag, Some(&test_invocation(vec![axis.clone()]))),
        Some(DexExecutionResult::Value(tag))
    );
    assert_eq!(
        program.invoke_builtin_method(&get_style, Some(&test_invocation(vec![axis.clone()]))),
        Some(DexExecutionResult::Value(DexValue::Float(1.0)))
    );
    assert_eq!(
        program.invoke_builtin_method(&get_tag_value, Some(&test_invocation(vec![axis.clone()]))),
        Some(DexExecutionResult::Int(0x77676874))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![
                axis.clone(),
                DexValue::String(std::rc::Rc::from("PRIV")),
                DexValue::Int((-1.0f32).to_bits() as i32)
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&get_style, Some(&test_invocation(vec![axis.clone()]))),
        Some(DexExecutionResult::Value(DexValue::Float(-1.0)))
    );
    for tag in [
        DexValue::Null,
        DexValue::String(std::rc::Rc::from("")),
        DexValue::String(std::rc::Rc::from("abcde")),
        DexValue::String(std::rc::Rc::from("\n\n\n\n")),
        DexValue::String(std::rc::Rc::from("あbcd")),
    ] {
        assert!(
            matches!(program.invoke_builtin_method(&constructor, Some(&test_invocation(vec![axis.clone(), tag, DexValue::Float(0.0)]))), Some(DexExecutionResult::Throw(DexValue::Object(exception))) if exception.class_descriptor.as_ref() == "Ljava/lang/IllegalArgumentException;")
        );
    }
}

#[test]
fn builtin_android_color_argb_packs_channels() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/graphics/Color;".to_owned(),
        name: "argb".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "IIIII".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec![
                "I".to_owned(),
                "I".to_owned(),
                "I".to_owned(),
                "I".to_owned(),
            ],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::Int(230),
                DexValue::Int(255),
                DexValue::Int(255),
                DexValue::Int(255),
            ])),
        ),
        Some(DexExecutionResult::Int(0xe6ff_ffffu32 as i32))
    );
}

#[test]
fn builtin_android_color_space_get_returns_named_color_space() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/graphics/ColorSpace;".to_owned(),
        name: "get".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Landroid/graphics/ColorSpace;".to_owned(),
            parameters: vec!["Landroid/graphics/ColorSpace$Named;".to_owned()],
        },
    };
    let named = DexValue::Reference(std::rc::Rc::from(
        "Landroid/graphics/ColorSpace$Named;.SRGB",
    ));

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![named]))),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from(
                "Landroid/graphics/ColorSpace;:reference:Landroid/graphics/ColorSpace$Named;.SRGB"
                    .to_owned()
            )
        )))
    );
}

#[test]
fn builtin_paint_style_values_returns_android_enum_order() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/graphics/Paint$Style;".to_owned(),
        name: "values".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "[Landroid/graphics/Paint$Style;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let ordinal = DexMethodId {
        class_descriptor: "Ljava/lang/Enum;".to_owned(),
        name: "ordinal".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };

    let Some(DexExecutionResult::Value(array)) = program.invoke_builtin_method(&method, None)
    else {
        panic!("Paint.Style.values should return an array");
    };
    assert_eq!(
        program
            .runtime
            .borrow()
            .array_entries
            .get(&map_identity_key(&array)),
        Some(&vec![
            DexValue::Reference(std::rc::Rc::from("Landroid/graphics/Paint$Style;.FILL")),
            DexValue::Reference(std::rc::Rc::from("Landroid/graphics/Paint$Style;.STROKE")),
            DexValue::Reference(std::rc::Rc::from(
                "Landroid/graphics/Paint$Style;.FILL_AND_STROKE"
            )),
        ])
    );
    assert_eq!(
        program.invoke_builtin_method(
            &ordinal,
            Some(&test_invocation(vec![DexValue::Reference(
                std::rc::Rc::from("Landroid/graphics/Paint$Style;.FILL_AND_STROKE")
            )]))
        ),
        Some(DexExecutionResult::Int(2))
    );
}

#[test]
fn builtin_paint_cap_values_returns_android_enum_order() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/graphics/Paint$Cap;".to_owned(),
        name: "values".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "[Landroid/graphics/Paint$Cap;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let ordinal = DexMethodId {
        class_descriptor: "Ljava/lang/Enum;".to_owned(),
        name: "ordinal".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };

    let Some(DexExecutionResult::Value(array)) = program.invoke_builtin_method(&method, None)
    else {
        panic!("Paint.Cap.values should return an array");
    };
    assert_eq!(
        program
            .runtime
            .borrow()
            .array_entries
            .get(&map_identity_key(&array)),
        Some(&vec![
            DexValue::Reference(std::rc::Rc::from("Landroid/graphics/Paint$Cap;.BUTT")),
            DexValue::Reference(std::rc::Rc::from("Landroid/graphics/Paint$Cap;.ROUND")),
            DexValue::Reference(std::rc::Rc::from("Landroid/graphics/Paint$Cap;.SQUARE")),
        ])
    );
    assert_eq!(
        program.invoke_builtin_method(
            &ordinal,
            Some(&test_invocation(vec![DexValue::Reference(
                std::rc::Rc::from("Landroid/graphics/Paint$Cap;.SQUARE")
            )]))
        ),
        Some(DexExecutionResult::Int(2))
    );
}

#[test]
fn builtin_paint_join_values_returns_android_enum_order() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/graphics/Paint$Join;".to_owned(),
        name: "values".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "[Landroid/graphics/Paint$Join;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let ordinal = DexMethodId {
        class_descriptor: "Ljava/lang/Enum;".to_owned(),
        name: "ordinal".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };

    let Some(DexExecutionResult::Value(array)) = program.invoke_builtin_method(&method, None)
    else {
        panic!("Paint.Join.values should return an array");
    };
    assert_eq!(
        program
            .runtime
            .borrow()
            .array_entries
            .get(&map_identity_key(&array)),
        Some(&vec![
            DexValue::Reference(std::rc::Rc::from("Landroid/graphics/Paint$Join;.MITER")),
            DexValue::Reference(std::rc::Rc::from("Landroid/graphics/Paint$Join;.ROUND")),
            DexValue::Reference(std::rc::Rc::from("Landroid/graphics/Paint$Join;.BEVEL")),
        ])
    );
    assert_eq!(
        program.invoke_builtin_method(
            &ordinal,
            Some(&test_invocation(vec![DexValue::Reference(
                std::rc::Rc::from("Landroid/graphics/Paint$Join;.BEVEL")
            )]))
        ),
        Some(DexExecutionResult::Int(2))
    );
}

#[test]
fn builtin_paint_stroke_properties_default_and_track_setters() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Landroid/graphics/Paint;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let get_stroke_width = DexMethodId {
        class_descriptor: "Landroid/graphics/Paint;".to_owned(),
        name: "getStrokeWidth".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "F".to_owned(),
            return_type: "F".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get_stroke_miter = DexMethodId {
        class_descriptor: "Landroid/graphics/Paint;".to_owned(),
        name: "getStrokeMiter".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "F".to_owned(),
            return_type: "F".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get_stroke_cap = DexMethodId {
        class_descriptor: "Landroid/graphics/Paint;".to_owned(),
        name: "getStrokeCap".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/graphics/Paint$Cap;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get_stroke_join = DexMethodId {
        class_descriptor: "Landroid/graphics/Paint;".to_owned(),
        name: "getStrokeJoin".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/graphics/Paint$Join;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set_stroke_width = DexMethodId {
        class_descriptor: "Landroid/graphics/Paint;".to_owned(),
        name: "setStrokeWidth".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VF".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["F".to_owned()],
        },
    };
    let set_stroke_miter = DexMethodId {
        class_descriptor: "Landroid/graphics/Paint;".to_owned(),
        name: "setStrokeMiter".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VF".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["F".to_owned()],
        },
    };
    let set_stroke_cap = DexMethodId {
        class_descriptor: "Landroid/graphics/Paint;".to_owned(),
        name: "setStrokeCap".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/graphics/Paint$Cap;".to_owned()],
        },
    };
    let set_stroke_join = DexMethodId {
        class_descriptor: "Landroid/graphics/Paint;".to_owned(),
        name: "setStrokeJoin".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/graphics/Paint$Join;".to_owned()],
        },
    };
    let paint = DexValue::Object(DexObjectRef {
        id: 7,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Paint;"),
    });
    let default_cap = DexValue::Reference(std::rc::Rc::from("Landroid/graphics/Paint$Cap;.BUTT"));
    let round_cap = DexValue::Reference(std::rc::Rc::from("Landroid/graphics/Paint$Cap;.ROUND"));
    let default_join =
        DexValue::Reference(std::rc::Rc::from("Landroid/graphics/Paint$Join;.MITER"));
    let bevel_join = DexValue::Reference(std::rc::Rc::from("Landroid/graphics/Paint$Join;.BEVEL"));

    assert_eq!(
        program.invoke_builtin_method(
            &get_stroke_width,
            Some(&test_invocation(vec![paint.clone()]))
        ),
        Some(DexExecutionResult::Value(DexValue::Float(0.0)))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_stroke_miter,
            Some(&test_invocation(vec![paint.clone()]))
        ),
        Some(DexExecutionResult::Value(DexValue::Float(4.0)))
    );
    assert_eq!(
        program.invoke_builtin_method(&get_stroke_cap, Some(&test_invocation(vec![paint.clone()]))),
        Some(DexExecutionResult::Value(default_cap.clone()))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_stroke_join,
            Some(&test_invocation(vec![paint.clone()]))
        ),
        Some(DexExecutionResult::Value(default_join.clone()))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![paint.clone(), DexValue::Int(3)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&paint, &paint_stroke_width_field()),
        Some(DexValue::Float(0.0))
    );
    assert_eq!(
        program.read_stored_instance_field(&paint, &paint_stroke_miter_field()),
        Some(DexValue::Float(4.0))
    );
    assert_eq!(
        program.read_stored_instance_field(&paint, &paint_stroke_cap_field()),
        Some(default_cap)
    );
    assert_eq!(
        program.read_stored_instance_field(&paint, &paint_stroke_join_field()),
        Some(default_join)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_stroke_width,
            Some(&test_invocation(vec![paint.clone(), DexValue::Float(2.5)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&get_stroke_width, Some(&test_invocation(vec![paint]))),
        Some(DexExecutionResult::Value(DexValue::Float(2.5)))
    );
    let paint = DexValue::Object(DexObjectRef {
        id: 7,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Paint;"),
    });
    assert_eq!(
        program.invoke_builtin_method(
            &set_stroke_miter,
            Some(&test_invocation(vec![paint.clone(), DexValue::Float(8.0)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&get_stroke_miter, Some(&test_invocation(vec![paint]))),
        Some(DexExecutionResult::Value(DexValue::Float(8.0)))
    );
    let paint = DexValue::Object(DexObjectRef {
        id: 7,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Paint;"),
    });
    assert_eq!(
        program.invoke_builtin_method(
            &set_stroke_cap,
            Some(&test_invocation(vec![paint.clone(), round_cap.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&get_stroke_cap, Some(&test_invocation(vec![paint]))),
        Some(DexExecutionResult::Value(round_cap))
    );
    let paint = DexValue::Object(DexObjectRef {
        id: 7,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Paint;"),
    });
    assert_eq!(
        program.invoke_builtin_method(
            &set_stroke_join,
            Some(&test_invocation(vec![paint.clone(), bevel_join.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&get_stroke_join, Some(&test_invocation(vec![paint]))),
        Some(DexExecutionResult::Value(bevel_join))
    );
}

#[test]
fn builtin_paint_get_text_locale_returns_stored_or_default_locale() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/graphics/Paint;".to_owned(),
        name: "getTextLocale".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/util/Locale;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let paint = DexValue::Object(DexObjectRef {
        id: 7,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Paint;"),
    });

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![paint.clone()]))),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Ljava/util/Locale;:default")
        )))
    );

    let locale = DexValue::Reference(std::rc::Rc::from("Ljava/util/Locale;:US"));
    assert!(program.write_stored_instance_field(
        &paint,
        &paint_text_locale_field(),
        locale.clone()
    ));

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![paint]))),
        Some(DexExecutionResult::Value(locale))
    );
}

#[test]
fn builtin_android_bitmap_create_bitmap_allocates_bitmap_metadata() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/graphics/Bitmap;".to_owned(),
        name: "createBitmap".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LLIILZL".to_owned(),
            return_type: "Landroid/graphics/Bitmap;".to_owned(),
            parameters: vec![
                "Landroid/util/DisplayMetrics;".to_owned(),
                "I".to_owned(),
                "I".to_owned(),
                "Landroid/graphics/Bitmap$Config;".to_owned(),
                "Z".to_owned(),
                "Landroid/graphics/ColorSpace;".to_owned(),
            ],
        },
    };
    let prepare_to_draw = DexMethodId {
        class_descriptor: "Landroid/graphics/Bitmap;".to_owned(),
        name: "prepareToDraw".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get_config = DexMethodId {
        class_descriptor: "Landroid/graphics/Bitmap;".to_owned(),
        name: "getConfig".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/graphics/Bitmap$Config;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get_width = DexMethodId {
        class_descriptor: "Landroid/graphics/Bitmap;".to_owned(),
        name: "getWidth".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get_height = DexMethodId {
        class_descriptor: "Landroid/graphics/Bitmap;".to_owned(),
        name: "getHeight".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let display_metrics =
        DexValue::Reference(std::rc::Rc::from("Landroid/util/DisplayMetrics;.bootstrap"));
    let config = DexValue::Reference(std::rc::Rc::from(
        "Landroid/graphics/Bitmap$Config;.ALPHA_8",
    ));
    let color_space = DexValue::Reference(std::rc::Rc::from(
        "Landroid/graphics/ColorSpace;:reference:Landroid/graphics/ColorSpace$Named;.SRGB"
            .to_owned(),
    ));

    let Some(DexExecutionResult::Value(bitmap)) = program.invoke_builtin_method(
        &method,
        Some(&test_invocation(vec![
            display_metrics.clone(),
            DexValue::Int(24),
            DexValue::Int(32),
            config.clone(),
            DexValue::Int(1),
            color_space.clone(),
        ])),
    ) else {
        panic!("Bitmap.createBitmap should allocate a bitmap");
    };

    assert_eq!(
        value_class_descriptor(&bitmap),
        Some("Landroid/graphics/Bitmap;")
    );
    assert_eq!(
        program.read_stored_instance_field(&bitmap, &bitmap_int_field("width")),
        Some(DexValue::Int(24))
    );
    assert_eq!(
        program.read_stored_instance_field(&bitmap, &bitmap_int_field("height")),
        Some(DexValue::Int(32))
    );
    assert_eq!(
        program.invoke_builtin_method(&get_width, Some(&test_invocation(vec![bitmap.clone()]))),
        Some(DexExecutionResult::Int(24))
    );
    assert_eq!(
        program.invoke_builtin_method(&get_height, Some(&test_invocation(vec![bitmap.clone()]))),
        Some(DexExecutionResult::Int(32))
    );
    assert_eq!(
        program.read_stored_instance_field(
            &bitmap,
            &bitmap_object_field("displayMetrics", "Landroid/util/DisplayMetrics;")
        ),
        Some(display_metrics)
    );
    assert_eq!(
        program.read_stored_instance_field(
            &bitmap,
            &bitmap_object_field("config", "Landroid/graphics/Bitmap$Config;")
        ),
        Some(config.clone())
    );
    assert_eq!(
        program.read_stored_instance_field(&bitmap, &bitmap_bool_field("mutable")),
        Some(DexValue::Int(1))
    );
    assert_eq!(
        program.read_stored_instance_field(
            &bitmap,
            &bitmap_object_field("colorSpace", "Landroid/graphics/ColorSpace;")
        ),
        Some(color_space)
    );
    assert_eq!(
        program.invoke_builtin_method(&get_config, Some(&test_invocation(vec![bitmap.clone()]))),
        Some(DexExecutionResult::Value(config))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &prepare_to_draw,
            Some(&test_invocation(vec![bitmap.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    let unknown_bitmap = DexValue::Object(DexObjectRef {
        id: 13,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Bitmap;"),
    });
    assert_eq!(
        program.invoke_builtin_method(
            &get_width,
            Some(&test_invocation(vec![unknown_bitmap.clone()]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&get_config, Some(&test_invocation(vec![unknown_bitmap]))),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Landroid/graphics/Bitmap$Config;.ARGB_8888")
        )))
    );
}

#[test]
fn builtin_canvas_bitmap_constructor_stores_bitmap_and_clip_bounds() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Landroid/graphics/Canvas;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/graphics/Bitmap;".to_owned()],
        },
    };
    let get_clip_bounds = DexMethodId {
        class_descriptor: "Landroid/graphics/Canvas;".to_owned(),
        name: "getClipBounds".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Landroid/graphics/Rect;".to_owned()],
        },
    };
    let canvas = DexValue::Object(DexObjectRef {
        id: 20,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Canvas;"),
    });
    let bitmap = DexValue::Object(DexObjectRef {
        id: 21,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Bitmap;"),
    });
    let clip_rect = DexValue::Object(DexObjectRef {
        id: 22,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Rect;"),
    });
    assert!(program.write_stored_instance_field(
        &bitmap,
        &bitmap_int_field("width"),
        DexValue::Int(48),
    ));
    assert!(program.write_stored_instance_field(
        &bitmap,
        &bitmap_int_field("height"),
        DexValue::Int(24),
    ));

    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![canvas.clone(), bitmap.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&canvas, &canvas_bitmap_field()),
        Some(bitmap)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_clip_bounds,
            Some(&test_invocation(vec![canvas, clip_rect.clone()]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.read_stored_instance_field(&clip_rect, &rect_int_field("right")),
        Some(DexValue::Int(48))
    );
    assert_eq!(
        program.read_stored_instance_field(&clip_rect, &rect_int_field("bottom")),
        Some(DexValue::Int(24))
    );
}

#[test]
fn builtin_canvas_draw_bitmap_at_records_full_bitmap_destination() {
    let program = DexProgram::new(Vec::new());
    let draw_bitmap_at = DexMethodId {
        class_descriptor: "Landroid/graphics/Canvas;".to_owned(),
        name: "drawBitmap".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VLFFL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "Landroid/graphics/Bitmap;".to_owned(),
                "F".to_owned(),
                "F".to_owned(),
                "Landroid/graphics/Paint;".to_owned(),
            ],
        },
    };
    let canvas = DexValue::Object(DexObjectRef {
        id: 23,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Canvas;"),
    });
    let bitmap = DexValue::Object(DexObjectRef {
        id: 24,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Bitmap;"),
    });
    assert!(program.write_stored_instance_field(
        &bitmap,
        &bitmap_int_field("width"),
        DexValue::Int(40),
    ));
    assert!(program.write_stored_instance_field(
        &bitmap,
        &bitmap_int_field("height"),
        DexValue::Int(20),
    ));

    assert_eq!(
        program.invoke_builtin_method(
            &draw_bitmap_at,
            Some(&test_invocation(vec![
                canvas.clone(),
                bitmap.clone(),
                DexValue::Float(12.0),
                DexValue::Float(16.0),
                DexValue::Null,
            ]))
        ),
        Some(DexExecutionResult::Void)
    );

    let ops = program.runtime.borrow().canvas_draw_ops[&map_identity_key(&canvas)].clone();
    assert_eq!(ops.len(), 1);
    let CanvasDrawOp::Bitmap {
        bitmap: recorded_bitmap,
        source,
        destination,
        paint,
    } = &ops[0]
    else {
        panic!("drawBitmap(Bitmap, float, float, Paint) should record a bitmap op");
    };
    assert_eq!(recorded_bitmap, &bitmap);
    assert_eq!(source, &DexValue::Null);
    assert_eq!(paint, &DexValue::Null);
    assert_eq!(
        program.read_stored_instance_field(destination, &rectf_float_field("left")),
        Some(DexValue::Float(12.0))
    );
    assert_eq!(
        program.read_stored_instance_field(destination, &rectf_float_field("bottom")),
        Some(DexValue::Float(36.0))
    );
}

#[test]
fn builtin_android_graphics_insets_of_stores_edges() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/graphics/Insets;".to_owned(),
        name: "of".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LIIII".to_owned(),
            return_type: "Landroid/graphics/Insets;".to_owned(),
            parameters: vec![
                "I".to_owned(),
                "I".to_owned(),
                "I".to_owned(),
                "I".to_owned(),
            ],
        },
    };
    let Some(DexExecutionResult::Value(insets)) = program.invoke_builtin_method(
        &method,
        Some(&test_invocation(vec![
            DexValue::Int(1),
            DexValue::Int(2),
            DexValue::Int(3),
            DexValue::Int(4),
        ])),
    ) else {
        panic!("Insets.of should return an Insets value");
    };

    assert_eq!(
        program.read_stored_instance_field(&insets, &insets_int_field("left")),
        Some(DexValue::Int(1))
    );
    assert_eq!(
        program.read_stored_instance_field(&insets, &insets_int_field("top")),
        Some(DexValue::Int(2))
    );
    assert_eq!(
        program.read_stored_instance_field(&insets, &insets_int_field("right")),
        Some(DexValue::Int(3))
    );
    assert_eq!(
        program.read_stored_instance_field(&insets, &insets_int_field("bottom")),
        Some(DexValue::Int(4))
    );
}

#[test]
fn builtin_canvas_no_arg_constructor_is_noop() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Landroid/graphics/Canvas;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let translate = DexMethodId {
        class_descriptor: "Landroid/graphics/Canvas;".to_owned(),
        name: "translate".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VFF".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["F".to_owned(), "F".to_owned()],
        },
    };
    let scale = DexMethodId {
        class_descriptor: "Landroid/graphics/Canvas;".to_owned(),
        name: "scale".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VFF".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["F".to_owned(), "F".to_owned()],
        },
    };
    let rotate = DexMethodId {
        class_descriptor: "Landroid/graphics/Canvas;".to_owned(),
        name: "rotate".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VF".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["F".to_owned()],
        },
    };
    let save = DexMethodId {
        class_descriptor: "Landroid/graphics/Canvas;".to_owned(),
        name: "save".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let save_layer = DexMethodId {
        class_descriptor: "Landroid/graphics/Canvas;".to_owned(),
        name: "saveLayer".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "IFFFFL".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec![
                "F".to_owned(),
                "F".to_owned(),
                "F".to_owned(),
                "F".to_owned(),
                "Landroid/graphics/Paint;".to_owned(),
            ],
        },
    };
    let restore = DexMethodId {
        class_descriptor: "Landroid/graphics/Canvas;".to_owned(),
        name: "restore".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let enable_z = DexMethodId {
        class_descriptor: "Landroid/graphics/Canvas;".to_owned(),
        name: "enableZ".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let disable_z = DexMethodId {
        class_descriptor: "Landroid/graphics/Canvas;".to_owned(),
        name: "disableZ".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let concat = DexMethodId {
        class_descriptor: "Landroid/graphics/Canvas;".to_owned(),
        name: "concat".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/graphics/Matrix;".to_owned()],
        },
    };
    let clip_rect_with_op = DexMethodId {
        class_descriptor: "Landroid/graphics/Canvas;".to_owned(),
        name: "clipRect".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZFFFFL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec![
                "F".to_owned(),
                "F".to_owned(),
                "F".to_owned(),
                "F".to_owned(),
                "Landroid/graphics/Region$Op;".to_owned(),
            ],
        },
    };
    let clip_path = DexMethodId {
        class_descriptor: "Landroid/graphics/Canvas;".to_owned(),
        name: "clipPath".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZLL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec![
                "Landroid/graphics/Path;".to_owned(),
                "Landroid/graphics/Region$Op;".to_owned(),
            ],
        },
    };
    let is_hardware_accelerated = DexMethodId {
        class_descriptor: "Landroid/graphics/Canvas;".to_owned(),
        name: "isHardwareAccelerated".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get_clip_bounds = DexMethodId {
        class_descriptor: "Landroid/graphics/Canvas;".to_owned(),
        name: "getClipBounds".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Landroid/graphics/Rect;".to_owned()],
        },
    };
    let draw_rect = DexMethodId {
        class_descriptor: "Landroid/graphics/Canvas;".to_owned(),
        name: "drawRect".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VFFFFL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "F".to_owned(),
                "F".to_owned(),
                "F".to_owned(),
                "F".to_owned(),
                "Landroid/graphics/Paint;".to_owned(),
            ],
        },
    };
    let draw_line = DexMethodId {
        class_descriptor: "Landroid/graphics/Canvas;".to_owned(),
        name: "drawLine".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VFFFFL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "F".to_owned(),
                "F".to_owned(),
                "F".to_owned(),
                "F".to_owned(),
                "Landroid/graphics/Paint;".to_owned(),
            ],
        },
    };
    let draw_circle = DexMethodId {
        class_descriptor: "Landroid/graphics/Canvas;".to_owned(),
        name: "drawCircle".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VFFFL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "F".to_owned(),
                "F".to_owned(),
                "F".to_owned(),
                "Landroid/graphics/Paint;".to_owned(),
            ],
        },
    };
    let draw_round_rect = DexMethodId {
        class_descriptor: "Landroid/graphics/Canvas;".to_owned(),
        name: "drawRoundRect".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VFFFFFFL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "F".to_owned(),
                "F".to_owned(),
                "F".to_owned(),
                "F".to_owned(),
                "F".to_owned(),
                "F".to_owned(),
                "Landroid/graphics/Paint;".to_owned(),
            ],
        },
    };
    let draw_path = DexMethodId {
        class_descriptor: "Landroid/graphics/Canvas;".to_owned(),
        name: "drawPath".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VLL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "Landroid/graphics/Path;".to_owned(),
                "Landroid/graphics/Paint;".to_owned(),
            ],
        },
    };
    let draw_bitmap = DexMethodId {
        class_descriptor: "Landroid/graphics/Canvas;".to_owned(),
        name: "drawBitmap".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VLLLL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "Landroid/graphics/Bitmap;".to_owned(),
                "Landroid/graphics/Rect;".to_owned(),
                "Landroid/graphics/Rect;".to_owned(),
                "Landroid/graphics/Paint;".to_owned(),
            ],
        },
    };
    let draw_render_node = DexMethodId {
        class_descriptor: "Landroid/graphics/Canvas;".to_owned(),
        name: "drawRenderNode".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/graphics/RenderNode;".to_owned()],
        },
    };
    let canvas = DexValue::Object(DexObjectRef {
        id: 1,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Canvas;"),
    });
    let paint = DexValue::Object(DexObjectRef {
        id: 2,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Paint;"),
    });
    let clip_rect = DexValue::Object(DexObjectRef {
        id: 3,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Rect;"),
    });
    let render_node = DexValue::Object(DexObjectRef {
        id: 4,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/RenderNode;"),
    });
    let path = DexValue::Object(DexObjectRef {
        id: 5,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Path;"),
    });
    let bitmap = DexValue::Object(DexObjectRef {
        id: 6,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Bitmap;"),
    });
    let source_rect = DexValue::Object(DexObjectRef {
        id: 7,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Rect;"),
    });
    let destination_rect = DexValue::Object(DexObjectRef {
        id: 8,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Rect;"),
    });
    let matrix = DexValue::Object(DexObjectRef {
        id: 9,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Matrix;"),
    });
    let region_op =
        DexValue::Reference(std::rc::Rc::from("Landroid/graphics/Region$Op;.INTERSECT"));

    assert_eq!(
        program.invoke_builtin_method(&constructor, Some(&test_invocation(vec![canvas.clone()]))),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &translate,
            Some(&test_invocation(vec![
                canvas.clone(),
                DexValue::Float(4.0),
                DexValue::Float(8.0),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &scale,
            Some(&test_invocation(vec![
                canvas.clone(),
                DexValue::Float(2.0),
                DexValue::Float(3.0),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&canvas, &canvas_float_field("scaleX")),
        Some(DexValue::Float(2.0))
    );
    assert_eq!(
        program.read_stored_instance_field(&canvas, &canvas_float_field("scaleY")),
        Some(DexValue::Float(3.0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &scale,
            Some(&test_invocation(vec![
                canvas.clone(),
                DexValue::Float(0.5),
                DexValue::Float(0.25),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&canvas, &canvas_float_field("scaleX")),
        Some(DexValue::Float(1.0))
    );
    assert_eq!(
        program.read_stored_instance_field(&canvas, &canvas_float_field("scaleY")),
        Some(DexValue::Float(0.75))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &rotate,
            Some(&test_invocation(vec![
                canvas.clone(),
                DexValue::Float(45.0),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&save, Some(&test_invocation(vec![canvas.clone()]))),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&save, Some(&test_invocation(vec![canvas.clone()]))),
        Some(DexExecutionResult::Int(2))
    );
    assert_eq!(
        program.read_stored_instance_field(&canvas, &canvas_save_count_field()),
        Some(DexValue::Int(3))
    );
    assert_eq!(
        program.invoke_builtin_method(&restore, Some(&test_invocation(vec![canvas.clone()]))),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&canvas, &canvas_save_count_field()),
        Some(DexValue::Int(2))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &save_layer,
            Some(&test_invocation(vec![
                canvas.clone(),
                DexValue::Float(0.0),
                DexValue::Float(1.0),
                DexValue::Float(20.0),
                DexValue::Float(30.0),
                paint.clone()
            ]))
        ),
        Some(DexExecutionResult::Int(2))
    );
    assert_eq!(
        program.read_stored_instance_field(&canvas, &canvas_save_count_field()),
        Some(DexValue::Int(3))
    );
    assert_eq!(
        program.invoke_builtin_method(&enable_z, Some(&test_invocation(vec![canvas.clone()]))),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&canvas, &canvas_bool_field("zEnabled")),
        Some(DexValue::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&disable_z, Some(&test_invocation(vec![canvas.clone()]))),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&canvas, &canvas_bool_field("zEnabled")),
        Some(DexValue::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &is_hardware_accelerated,
            Some(&test_invocation(vec![canvas.clone()]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    // cite: frameworks-base Canvas.java#Canvas, getWidth, getHeight, getClipBounds
    assert_eq!(
        program.invoke_builtin_method(
            &get_clip_bounds,
            Some(&test_invocation(vec![canvas.clone(), clip_rect.clone()]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.read_stored_instance_field(&clip_rect, &rect_int_field("right")),
        Some(DexValue::Int(0))
    );
    assert_eq!(
        program.read_stored_instance_field(&clip_rect, &rect_int_field("bottom")),
        Some(DexValue::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &concat,
            Some(&test_invocation(vec![canvas.clone(), matrix.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &clip_rect_with_op,
            Some(&test_invocation(vec![
                canvas.clone(),
                DexValue::Float(0.0),
                DexValue::Float(1.0),
                DexValue::Float(20.0),
                DexValue::Float(30.0),
                region_op.clone(),
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &clip_path,
            Some(&test_invocation(vec![
                canvas.clone(),
                path.clone(),
                region_op.clone(),
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &draw_line,
            Some(&test_invocation(vec![
                canvas.clone(),
                DexValue::Float(11.0),
                DexValue::Float(12.0),
                DexValue::Float(13.0),
                DexValue::Float(14.0),
                paint.clone(),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &draw_circle,
            Some(&test_invocation(vec![
                canvas.clone(),
                DexValue::Float(21.0),
                DexValue::Float(22.0),
                DexValue::Float(23.0),
                paint.clone(),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &draw_rect,
            Some(&test_invocation(vec![
                canvas.clone(),
                DexValue::Float(1.0),
                DexValue::Float(2.0),
                DexValue::Float(3.0),
                DexValue::Float(4.0),
                paint.clone(),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &draw_round_rect,
            Some(&test_invocation(vec![
                canvas.clone(),
                DexValue::Float(5.0),
                DexValue::Float(6.0),
                DexValue::Float(7.0),
                DexValue::Float(8.0),
                DexValue::Float(9.0),
                DexValue::Float(10.0),
                paint.clone(),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &draw_path,
            Some(&test_invocation(vec![
                canvas.clone(),
                path.clone(),
                paint.clone(),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &draw_bitmap,
            Some(&test_invocation(vec![
                canvas.clone(),
                bitmap.clone(),
                source_rect.clone(),
                destination_rect.clone(),
                paint.clone(),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &draw_render_node,
            Some(&test_invocation(vec![canvas.clone(), render_node.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .canvas_draw_ops
            .get(&map_identity_key(&canvas))
            .cloned(),
        Some(vec![
            CanvasDrawOp::Translate { dx: 4.0, dy: 8.0 },
            CanvasDrawOp::Scale { sx: 2.0, sy: 3.0 },
            CanvasDrawOp::Scale { sx: 0.5, sy: 0.25 },
            CanvasDrawOp::Rotate { degrees: 45.0 },
            CanvasDrawOp::Save,
            CanvasDrawOp::Save,
            CanvasDrawOp::Restore,
            CanvasDrawOp::Save,
            CanvasDrawOp::ConcatMatrix { matrix },
            CanvasDrawOp::ClipRect {
                left: 0.0,
                top: 1.0,
                right: 20.0,
                bottom: 30.0,
                op: region_op,
            },
            CanvasDrawOp::ClipPath {
                ops: Vec::new(),
                op: DexValue::Reference(std::rc::Rc::from(
                    "Landroid/graphics/Region$Op;.INTERSECT"
                )),
            },
            CanvasDrawOp::Line {
                x1: 11.0,
                y1: 12.0,
                x2: 13.0,
                y2: 14.0,
                paint: paint.clone(),
            },
            CanvasDrawOp::Circle {
                cx: 21.0,
                cy: 22.0,
                radius: 23.0,
                paint: paint.clone(),
            },
            CanvasDrawOp::Rect {
                left: 1.0,
                top: 2.0,
                right: 3.0,
                bottom: 4.0,
                paint: paint.clone(),
            },
            CanvasDrawOp::RoundRect {
                left: 5.0,
                top: 6.0,
                right: 7.0,
                bottom: 8.0,
                rx: 9.0,
                ry: 10.0,
                paint: paint.clone(),
            },
            CanvasDrawOp::Path {
                ops: Vec::new(),
                paint: paint.clone(),
            },
            CanvasDrawOp::Bitmap {
                bitmap,
                source: source_rect,
                destination: destination_rect,
                paint,
            },
            CanvasDrawOp::RenderNode { render_node },
        ])
    );
}

#[test]
fn view_snapshot_replays_bitmap_backing_canvas_ops() {
    let program = DexProgram::new(Vec::new());
    let root_canvas = DexValue::Object(DexObjectRef {
        id: 300,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Canvas;"),
    });
    let bitmap_canvas = DexValue::Object(DexObjectRef {
        id: 301,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Canvas;"),
    });
    let bitmap = DexValue::Object(DexObjectRef {
        id: 302,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Bitmap;"),
    });
    let destination = DexValue::Object(DexObjectRef {
        id: 303,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Rect;"),
    });
    let paint = DexValue::Object(DexObjectRef {
        id: 304,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Paint;"),
    });

    {
        let mut runtime = program.runtime.borrow_mut();
        for (id, descriptor) in [
            (300, "Landroid/graphics/Canvas;"),
            (301, "Landroid/graphics/Canvas;"),
            (302, "Landroid/graphics/Bitmap;"),
            (303, "Landroid/graphics/Rect;"),
            (304, "Landroid/graphics/Paint;"),
        ] {
            runtime
                .instance_fields
                .register_object_id(id, descriptor.to_owned());
        }
    }

    assert!(program.write_stored_instance_field(
        &bitmap,
        &bitmap_int_field("width"),
        DexValue::Int(40),
    ));
    assert!(program.write_stored_instance_field(
        &bitmap,
        &bitmap_int_field("height"),
        DexValue::Int(20),
    ));
    assert!(program.write_stored_instance_field(
        &bitmap_canvas,
        &canvas_bitmap_field(),
        bitmap.clone(),
    ));
    assert!(program.write_stored_instance_field(
        &destination,
        &rect_int_field("left"),
        DexValue::Int(10),
    ));
    assert!(program.write_stored_instance_field(
        &destination,
        &rect_int_field("top"),
        DexValue::Int(12),
    ));
    assert!(program.write_stored_instance_field(
        &destination,
        &rect_int_field("right"),
        DexValue::Int(90),
    ));
    assert!(program.write_stored_instance_field(
        &destination,
        &rect_int_field("bottom"),
        DexValue::Int(52),
    ));
    assert!(program.write_stored_instance_field(
        &paint,
        &paint_color_field(),
        DexValue::Int(0xff11_2233u32 as i32),
    ));

    {
        let mut runtime = program.runtime.borrow_mut();
        runtime.canvas_draw_ops.insert(
            map_identity_key(&bitmap_canvas),
            vec![CanvasDrawOp::Rect {
                left: 0.0,
                top: 0.0,
                right: 40.0,
                bottom: 20.0,
                paint: paint.clone(),
            }],
        );
        runtime.canvas_draw_ops.insert(
            map_identity_key(&root_canvas),
            vec![CanvasDrawOp::Bitmap {
                bitmap: bitmap.clone(),
                source: DexValue::Null,
                destination,
                paint,
            }],
        );
        runtime.runtime_canvas_frames.push(RuntimeCanvasFrame {
            x: 0,
            y: 0,
            root_view_key: "object:root".to_owned(),
            class_descriptor: "Landroid/view/View;".to_owned(),
            width: 100,
            height: 80,
            canvas_key: map_identity_key(&root_canvas),
        });
    }

    // cite: frameworks-base BaseCanvas.java#drawBitmap
    let snapshot = program.view_snapshot();
    assert_eq!(
        snapshot.runtime_canvas_frames[0].ops,
        vec![DexRuntimeCanvasOp::Bitmap {
            left: 10.0,
            top: 12.0,
            right: 90.0,
            bottom: 52.0,
            source_left: 0.0,
            source_top: 0.0,
            source_right: 40.0,
            source_bottom: 20.0,
            bitmap_width: 40,
            bitmap_height: 20,
            color: 0xff11_2233,
            blend_mode: DexRuntimeCanvasBlendMode::SrcOver,
            color_filter: None,
            alpha_mask: false,
            encoded_image: None,
            ops: vec![DexRuntimeCanvasOp::Rect {
                left: 0.0,
                top: 0.0,
                right: 40.0,
                bottom: 20.0,
                color: 0xff11_2233,
                blend_mode: DexRuntimeCanvasBlendMode::SrcOver,
                paint_style: DexRuntimeCanvasPaintStyle::Fill,
            }],
        }]
    );
}

#[test]
fn view_snapshot_bitmap_op_carries_blend_mode_color_filter_and_alpha_mask() {
    let program = DexProgram::new(Vec::new());
    let root_canvas = DexValue::Object(DexObjectRef {
        id: 310,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Canvas;"),
    });
    let bitmap = DexValue::Object(DexObjectRef {
        id: 311,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Bitmap;"),
    });
    let destination = DexValue::Object(DexObjectRef {
        id: 312,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Rect;"),
    });
    let paint = DexValue::Object(DexObjectRef {
        id: 313,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Paint;"),
    });
    let color_filter = DexValue::Object(DexObjectRef {
        id: 314,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/BlendModeColorFilter;"),
    });

    assert!(program.write_stored_instance_field(
        &bitmap,
        &bitmap_int_field("width"),
        DexValue::Int(24),
    ));
    assert!(program.write_stored_instance_field(
        &bitmap,
        &bitmap_int_field("height"),
        DexValue::Int(24),
    ));
    // cite: frameworks-base Bitmap.java#ALPHA_8
    assert!(program.write_stored_instance_field(
        &bitmap,
        &bitmap_object_field("config", "Landroid/graphics/Bitmap$Config;"),
        DexValue::Reference(std::rc::Rc::from(
            "Landroid/graphics/Bitmap$Config;.ALPHA_8"
        )),
    ));
    for (name, value) in [("left", 0), ("top", 0), ("right", 48), ("bottom", 48)] {
        assert!(program.write_stored_instance_field(
            &destination,
            &rect_int_field(name),
            DexValue::Int(value),
        ));
    }
    // cite: frameworks-base BlendModeColorFilter.java#BlendModeColorFilter
    assert!(program.write_stored_instance_field(
        &color_filter,
        &blend_mode_color_filter_color_field(),
        DexValue::Int(0xff40_5060u32 as i32),
    ));
    assert!(program.write_stored_instance_field(
        &color_filter,
        &blend_mode_color_filter_mode_field(),
        DexValue::Reference(std::rc::Rc::from("Landroid/graphics/BlendMode;.SRC_IN")),
    ));
    assert!(program.write_stored_instance_field(
        &paint,
        &paint_color_field(),
        DexValue::Int(0xffff_ffffu32 as i32),
    ));
    assert!(program.write_stored_instance_field(&paint, &paint_color_filter_field(), color_filter,));

    {
        let mut runtime = program.runtime.borrow_mut();
        runtime.canvas_draw_ops.insert(
            map_identity_key(&root_canvas),
            vec![CanvasDrawOp::Bitmap {
                bitmap,
                source: DexValue::Null,
                destination,
                paint,
            }],
        );
        runtime.runtime_canvas_frames.push(RuntimeCanvasFrame {
            x: 0,
            y: 0,
            root_view_key: "object:root".to_owned(),
            class_descriptor: "Landroid/view/View;".to_owned(),
            width: 100,
            height: 100,
            canvas_key: map_identity_key(&root_canvas),
        });
    }

    let snapshot = program.view_snapshot();
    assert_eq!(
        snapshot.runtime_canvas_frames[0].ops,
        vec![DexRuntimeCanvasOp::Bitmap {
            left: 0.0,
            top: 0.0,
            right: 48.0,
            bottom: 48.0,
            source_left: 0.0,
            source_top: 0.0,
            source_right: 24.0,
            source_bottom: 24.0,
            bitmap_width: 24,
            bitmap_height: 24,
            color: 0xffff_ffff,
            blend_mode: DexRuntimeCanvasBlendMode::SrcOver,
            color_filter: Some(DexRuntimeCanvasColorFilter {
                color: 0xff40_5060,
                blend_mode: DexRuntimeCanvasBlendMode::SrcIn,
            }),
            alpha_mask: true,
            encoded_image: None,
            ops: Vec::new(),
        }]
    );
}

#[test]
fn view_snapshot_bitmap_op_null_paint_uses_draw_bitmap_defaults() {
    let program = DexProgram::new(Vec::new());
    let root_canvas = DexValue::Object(DexObjectRef {
        id: 320,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Canvas;"),
    });
    let bitmap = DexValue::Object(DexObjectRef {
        id: 321,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Bitmap;"),
    });
    let destination = DexValue::Object(DexObjectRef {
        id: 322,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Rect;"),
    });

    assert!(program.write_stored_instance_field(
        &bitmap,
        &bitmap_int_field("width"),
        DexValue::Int(8),
    ));
    assert!(program.write_stored_instance_field(
        &bitmap,
        &bitmap_int_field("height"),
        DexValue::Int(8),
    ));
    for (name, value) in [("left", 0), ("top", 0), ("right", 8), ("bottom", 8)] {
        assert!(program.write_stored_instance_field(
            &destination,
            &rect_int_field(name),
            DexValue::Int(value),
        ));
    }

    {
        let mut runtime = program.runtime.borrow_mut();
        runtime.canvas_draw_ops.insert(
            map_identity_key(&root_canvas),
            vec![CanvasDrawOp::Bitmap {
                bitmap,
                source: DexValue::Null,
                destination,
                paint: DexValue::Null,
            }],
        );
        runtime.runtime_canvas_frames.push(RuntimeCanvasFrame {
            x: 0,
            y: 0,
            root_view_key: "object:root".to_owned(),
            class_descriptor: "Landroid/view/View;".to_owned(),
            width: 100,
            height: 100,
            canvas_key: map_identity_key(&root_canvas),
        });
    }

    // cite: frameworks-base BaseCanvas.java#drawBitmap
    let snapshot = program.view_snapshot();
    assert_eq!(
        snapshot.runtime_canvas_frames[0].ops,
        vec![DexRuntimeCanvasOp::Bitmap {
            left: 0.0,
            top: 0.0,
            right: 8.0,
            bottom: 8.0,
            source_left: 0.0,
            source_top: 0.0,
            source_right: 8.0,
            source_bottom: 8.0,
            bitmap_width: 8,
            bitmap_height: 8,
            color: 0xff00_0000,
            blend_mode: DexRuntimeCanvasBlendMode::SrcOver,
            color_filter: None,
            alpha_mask: false,
            encoded_image: None,
            ops: Vec::new(),
        }]
    );
}

#[test]
fn view_snapshot_bitmap_op_carries_bitmapfactory_decoded_resource_bytes() {
    let program = DexProgram::new(Vec::new());
    let root_canvas = DexValue::Object(DexObjectRef {
        id: 323,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Canvas;"),
    });
    let bitmap = DexValue::Object(DexObjectRef {
        id: 324,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Bitmap;"),
    });
    let destination = DexValue::Object(DexObjectRef {
        id: 325,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Rect;"),
    });
    let encoded = vec![0x89, b'P', b'N', b'G'];

    assert!(program.write_stored_instance_field(
        &bitmap,
        &bitmap_int_field("width"),
        DexValue::Int(1),
    ));
    assert!(program.write_stored_instance_field(
        &bitmap,
        &bitmap_int_field("height"),
        DexValue::Int(1),
    ));
    for (name, value) in [("left", 4), ("top", 6), ("right", 12), ("bottom", 14)] {
        assert!(program.write_stored_instance_field(
            &destination,
            &rect_int_field(name),
            DexValue::Int(value),
        ));
    }

    {
        let mut runtime = program.runtime.borrow_mut();
        runtime
            .bitmap_encoded_images
            .insert(map_identity_key(&bitmap), encoded.clone());
        runtime.canvas_draw_ops.insert(
            map_identity_key(&root_canvas),
            vec![CanvasDrawOp::Bitmap {
                bitmap,
                source: DexValue::Null,
                destination,
                paint: DexValue::Null,
            }],
        );
        runtime.runtime_canvas_frames.push(RuntimeCanvasFrame {
            x: 0,
            y: 0,
            root_view_key: "object:root".to_owned(),
            class_descriptor: "Landroid/view/View;".to_owned(),
            width: 20,
            height: 20,
            canvas_key: map_identity_key(&root_canvas),
        });
    }

    // cite: frameworks-base BitmapFactory.java#decodeResource, decodeResourceStream
    let snapshot = program.view_snapshot();
    assert_eq!(
        snapshot.runtime_canvas_frames[0].ops,
        vec![DexRuntimeCanvasOp::Bitmap {
            left: 4.0,
            top: 6.0,
            right: 12.0,
            bottom: 14.0,
            source_left: 0.0,
            source_top: 0.0,
            source_right: 1.0,
            source_bottom: 1.0,
            bitmap_width: 1,
            bitmap_height: 1,
            color: 0xff00_0000,
            blend_mode: DexRuntimeCanvasBlendMode::SrcOver,
            color_filter: None,
            alpha_mask: false,
            encoded_image: Some(encoded),
            ops: Vec::new(),
        }]
    );
}

#[test]
fn view_snapshot_replays_line_with_paint_stroke_state() {
    let program = DexProgram::new(Vec::new());
    let canvas = DexValue::Object(DexObjectRef {
        id: 325,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Canvas;"),
    });
    let paint = DexValue::Object(DexObjectRef {
        id: 326,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Paint;"),
    });

    assert!(program.write_stored_instance_field(
        &paint,
        &paint_color_field(),
        DexValue::Int(0xff44_5566u32 as i32),
    ));
    assert!(program.write_stored_instance_field(
        &paint,
        &paint_stroke_width_field(),
        DexValue::Float(3.5),
    ));
    assert!(program.write_stored_instance_field(
        &paint,
        &paint_stroke_cap_field(),
        DexValue::Reference(std::rc::Rc::from("Landroid/graphics/Paint$Cap;.ROUND")),
    ));

    {
        let mut runtime = program.runtime.borrow_mut();
        runtime.canvas_draw_ops.insert(
            map_identity_key(&canvas),
            vec![CanvasDrawOp::Line {
                x1: 1.0,
                y1: 2.0,
                x2: 30.0,
                y2: 40.0,
                paint,
            }],
        );
        runtime.runtime_canvas_frames.push(RuntimeCanvasFrame {
            x: 0,
            y: 0,
            root_view_key: "object:root".to_owned(),
            class_descriptor: "Landroid/view/View;".to_owned(),
            width: 48,
            height: 48,
            canvas_key: map_identity_key(&canvas),
        });
    }

    let snapshot = program.view_snapshot();
    assert_eq!(
        snapshot.runtime_canvas_frames[0].ops,
        vec![DexRuntimeCanvasOp::Line {
            x1: 1.0,
            y1: 2.0,
            x2: 30.0,
            y2: 40.0,
            color: 0xff44_5566,
            blend_mode: DexRuntimeCanvasBlendMode::SrcOver,
            stroke_width: 3.5,
            stroke_cap: DexRuntimeCanvasStrokeCap::Round,
        }]
    );
}

#[test]
fn aosp_render_node_snapshot_carries_position_alpha_scale_and_clip_properties() {
    let program = DexProgram::new(Vec::new());
    let root_canvas = DexValue::Object(DexObjectRef {
        id: 430,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Canvas;"),
    });
    let render_node = DexValue::Object(DexObjectRef {
        id: 431,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/RenderNode;"),
    });
    let paint = DexValue::Object(DexObjectRef {
        id: 432,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Paint;"),
    });

    for (name, value) in [
        ("positionLeft", 3),
        ("positionTop", 4),
        ("positionRight", 43),
        ("positionBottom", 24),
    ] {
        assert!(program.write_stored_instance_field(
            &render_node,
            &render_node_position_field(name),
            DexValue::Int(value),
        ));
    }
    assert!(program.write_stored_instance_field(
        &render_node,
        &render_node_alpha_field(),
        DexValue::Float(0.5),
    ));
    assert!(program.write_stored_instance_field(
        &render_node,
        &render_node_scale_x_field(),
        DexValue::Float(2.0),
    ));
    assert!(program.write_stored_instance_field(
        &render_node,
        &render_node_scale_y_field(),
        DexValue::Float(0.25),
    ));
    assert!(program.write_stored_instance_field(
        &render_node,
        &render_node_clip_to_bounds_field(),
        DexValue::Int(0),
    ));
    assert!(program.write_stored_instance_field(
        &paint,
        &paint_color_field(),
        DexValue::Int(0xff44_5566u32 as i32),
    ));

    {
        let mut runtime = program.runtime.borrow_mut();
        runtime.render_node_draw_ops.insert(
            map_identity_key(&render_node),
            vec![CanvasDrawOp::Rect {
                left: 1.0,
                top: 2.0,
                right: 10.0,
                bottom: 12.0,
                paint,
            }],
        );
        runtime.canvas_draw_ops.insert(
            map_identity_key(&root_canvas),
            vec![CanvasDrawOp::RenderNode {
                render_node: render_node.clone(),
            }],
        );
        runtime.runtime_canvas_frames.push(RuntimeCanvasFrame {
            x: 0,
            y: 0,
            root_view_key: "object:root".to_owned(),
            class_descriptor: "Landroid/view/View;".to_owned(),
            width: 100,
            height: 80,
            canvas_key: map_identity_key(&root_canvas),
        });
    }

    let snapshot = program.view_snapshot();
    assert_eq!(
        snapshot.runtime_canvas_frames[0].ops,
        vec![DexRuntimeCanvasOp::RenderNode {
            left: 3.0,
            top: 4.0,
            right: 43.0,
            bottom: 24.0,
            alpha: 0.5,
            scale_x: 2.0,
            scale_y: 0.25,
            clip_to_bounds: false,
            ops: vec![DexRuntimeCanvasOp::Rect {
                left: 1.0,
                top: 2.0,
                right: 10.0,
                bottom: 12.0,
                color: 0xff44_5566,
                blend_mode: DexRuntimeCanvasBlendMode::SrcOver,
                paint_style: DexRuntimeCanvasPaintStyle::Fill,
            }],
        }]
    );
}

#[test]
fn view_snapshot_freezes_mutable_paint_at_draw_time() {
    let program = DexProgram::new(Vec::new());
    let draw_rect = DexMethodId {
        class_descriptor: "Landroid/graphics/Canvas;".to_owned(),
        name: "drawRect".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VFFFFL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "F".to_owned(),
                "F".to_owned(),
                "F".to_owned(),
                "F".to_owned(),
                "Landroid/graphics/Paint;".to_owned(),
            ],
        },
    };
    let canvas = DexValue::Object(DexObjectRef {
        id: 330,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Canvas;"),
    });
    let paint = DexValue::Object(DexObjectRef {
        id: 331,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Paint;"),
    });

    assert!(program.write_stored_instance_field(
        &paint,
        &paint_blend_mode_field(),
        DexValue::Reference(std::rc::Rc::from("Landroid/graphics/BlendMode;.CLEAR")),
    ));
    assert_eq!(
        program.invoke_builtin_method(
            &draw_rect,
            Some(&test_invocation(vec![
                canvas.clone(),
                DexValue::Float(0.0),
                DexValue::Float(0.0),
                DexValue::Float(24.0),
                DexValue::Float(24.0),
                paint.clone(),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert!(program.write_stored_instance_field(
        &paint,
        &paint_blend_mode_field(),
        DexValue::Reference(std::rc::Rc::from("Landroid/graphics/BlendMode;.SRC_OVER")),
    ));
    program
        .runtime
        .borrow_mut()
        .runtime_canvas_frames
        .push(RuntimeCanvasFrame {
            x: 0,
            y: 0,
            root_view_key: "object:root".to_owned(),
            class_descriptor: "Landroid/view/View;".to_owned(),
            width: 24,
            height: 24,
            canvas_key: map_identity_key(&canvas),
        });

    let snapshot = program.view_snapshot();
    assert_eq!(
        snapshot.runtime_canvas_frames[0].ops,
        vec![DexRuntimeCanvasOp::Rect {
            left: 0.0,
            top: 0.0,
            right: 24.0,
            bottom: 24.0,
            color: 0xff00_0000,
            blend_mode: DexRuntimeCanvasBlendMode::Clear,
            paint_style: DexRuntimeCanvasPaintStyle::Fill,
        }]
    );
}

#[test]
fn view_snapshot_replays_path_ops() {
    let program = DexProgram::new(Vec::new());
    let root_canvas = DexValue::Object(DexObjectRef {
        id: 320,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Canvas;"),
    });
    let paint = DexValue::Object(DexObjectRef {
        id: 323,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Paint;"),
    });
    let clip_op = DexValue::Reference(std::rc::Rc::from("Landroid/graphics/Region$Op;.INTERSECT"));

    assert!(program.write_stored_instance_field(
        &paint,
        &paint_color_field(),
        DexValue::Int(0xff44_5566u32 as i32),
    ));

    let path_ops = vec![
        PathOp::MoveTo { x: 3.0, y: 4.0 },
        PathOp::LineTo { x: 13.0, y: 14.0 },
        PathOp::RLineTo { dx: 2.0, dy: 3.0 },
        PathOp::QuadTo {
            x1: 5.0,
            y1: 6.0,
            x2: 15.0,
            y2: 16.0,
        },
        PathOp::BooleanOp {
            left: vec![PathOp::Rect {
                left: 1.0,
                top: 2.0,
                right: 31.0,
                bottom: 42.0,
                clockwise: true,
            }],
            right: vec![PathOp::MoveTo { x: 17.0, y: 18.0 }],
            op: PathBooleanOp::Intersect,
        },
        PathOp::Rect {
            left: 1.0,
            top: 2.0,
            right: 31.0,
            bottom: 42.0,
            clockwise: true,
        },
        PathOp::RoundRect {
            left: 1.0,
            top: 2.0,
            right: 31.0,
            bottom: 42.0,
            radii: vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0],
            clockwise: true,
        },
        PathOp::Close,
    ];

    {
        let mut runtime = program.runtime.borrow_mut();
        runtime.canvas_draw_ops.insert(
            map_identity_key(&root_canvas),
            vec![
                CanvasDrawOp::ClipRect {
                    left: 0.0,
                    top: 0.0,
                    right: 50.0,
                    bottom: 60.0,
                    op: clip_op.clone(),
                },
                CanvasDrawOp::ClipPath {
                    ops: path_ops.clone(),
                    op: clip_op,
                },
                CanvasDrawOp::Path {
                    ops: path_ops,
                    paint,
                },
            ],
        );
        runtime.runtime_canvas_frames.push(RuntimeCanvasFrame {
            x: 0,
            y: 0,
            root_view_key: "object:root".to_owned(),
            class_descriptor: "Landroid/view/View;".to_owned(),
            width: 100,
            height: 80,
            canvas_key: map_identity_key(&root_canvas),
        });
    }

    let snapshot = program.view_snapshot();
    assert_eq!(
        snapshot.runtime_canvas_frames[0].ops,
        vec![
            DexRuntimeCanvasOp::ClipRect {
                left: 0.0,
                top: 0.0,
                right: 50.0,
                bottom: 60.0,
                op: DexRuntimeCanvasClipOp::Intersect,
            },
            DexRuntimeCanvasOp::ClipPath {
                ops: expected_runtime_canvas_path_ops(),
                op: DexRuntimeCanvasClipOp::Intersect,
            },
            DexRuntimeCanvasOp::Path {
                color: 0xff44_5566,
                blend_mode: DexRuntimeCanvasBlendMode::SrcOver,
                paint_style: DexRuntimeCanvasPaintStyle::Fill,
                ops: expected_runtime_canvas_path_ops(),
            },
        ]
    );
}

fn expected_runtime_canvas_path_ops() -> Vec<DexRuntimeCanvasPathOp> {
    vec![
        DexRuntimeCanvasPathOp::MoveTo { x: 3.0, y: 4.0 },
        DexRuntimeCanvasPathOp::LineTo { x: 13.0, y: 14.0 },
        DexRuntimeCanvasPathOp::RLineTo { dx: 2.0, dy: 3.0 },
        DexRuntimeCanvasPathOp::QuadTo {
            x1: 5.0,
            y1: 6.0,
            x2: 15.0,
            y2: 16.0,
        },
        DexRuntimeCanvasPathOp::BooleanOp {
            left: vec![DexRuntimeCanvasPathOp::Rect {
                left: 1.0,
                top: 2.0,
                right: 31.0,
                bottom: 42.0,
                clockwise: true,
            }],
            right: vec![DexRuntimeCanvasPathOp::MoveTo { x: 17.0, y: 18.0 }],
            op: DexRuntimeCanvasPathBooleanOp::Intersect,
        },
        DexRuntimeCanvasPathOp::Rect {
            left: 1.0,
            top: 2.0,
            right: 31.0,
            bottom: 42.0,
            clockwise: true,
        },
        DexRuntimeCanvasPathOp::RoundRect {
            left: 1.0,
            top: 2.0,
            right: 31.0,
            bottom: 42.0,
            radii: vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0],
            clockwise: true,
        },
        DexRuntimeCanvasPathOp::Close,
    ]
}

#[test]
fn builtin_outline_no_arg_constructor_is_noop() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Landroid/graphics/Outline;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set_round_rect = DexMethodId {
        class_descriptor: "Landroid/graphics/Outline;".to_owned(),
        name: "setRoundRect".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VIIIIF".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "I".to_owned(),
                "I".to_owned(),
                "I".to_owned(),
                "I".to_owned(),
                "F".to_owned(),
            ],
        },
    };
    let set_alpha = DexMethodId {
        class_descriptor: "Landroid/graphics/Outline;".to_owned(),
        name: "setAlpha".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VF".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["F".to_owned()],
        },
    };
    let outline = DexValue::Object(DexObjectRef {
        id: 2,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Outline;"),
    });

    assert_eq!(
        program.invoke_builtin_method(&constructor, Some(&test_invocation(vec![outline]))),
        Some(DexExecutionResult::Void)
    );
    let outline = DexValue::Object(DexObjectRef {
        id: 2,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Outline;"),
    });
    assert_eq!(
        program.invoke_builtin_method(
            &set_round_rect,
            Some(&test_invocation(vec![
                outline.clone(),
                DexValue::Int(1),
                DexValue::Int(2),
                DexValue::Int(3),
                DexValue::Int(4),
                DexValue::Float(5.5),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(
            &outline,
            &outline_field("roundRectLeft", &DexValue::Int(0))
        ),
        Some(DexValue::Int(1))
    );
    assert_eq!(
        program.read_stored_instance_field(
            &outline,
            &outline_field("roundRectBottom", &DexValue::Int(0))
        ),
        Some(DexValue::Int(4))
    );
    assert_eq!(
        program.read_stored_instance_field(
            &outline,
            &outline_field("roundRectRadius", &DexValue::Float(0.0))
        ),
        Some(DexValue::Float(5.5))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_alpha,
            Some(&test_invocation(vec![
                outline.clone(),
                DexValue::Float(0.75),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program
            .read_stored_instance_field(&outline, &outline_field("alpha", &DexValue::Float(0.0))),
        Some(DexValue::Float(0.75))
    );
}

#[test]
fn aosp_render_node_api29_layer_primitives_share_retained_display_list() {
    let program = DexProgram::new(Vec::new());
    let constructor = graphics_method(
        "Landroid/graphics/RenderNode;",
        "<init>",
        "V",
        &["Ljava/lang/String;"],
    );
    let set_position = graphics_method(
        "Landroid/graphics/RenderNode;",
        "setPosition",
        "Z",
        &["I", "I", "I", "I"],
    );
    let offset_left_and_right = graphics_method(
        "Landroid/graphics/RenderNode;",
        "offsetLeftAndRight",
        "Z",
        &["I"],
    );
    let offset_top_and_bottom = graphics_method(
        "Landroid/graphics/RenderNode;",
        "offsetTopAndBottom",
        "Z",
        &["I"],
    );
    let get_left = graphics_method("Landroid/graphics/RenderNode;", "getLeft", "I", &[]);
    let get_top = graphics_method("Landroid/graphics/RenderNode;", "getTop", "I", &[]);
    let get_right = graphics_method("Landroid/graphics/RenderNode;", "getRight", "I", &[]);
    let get_bottom = graphics_method("Landroid/graphics/RenderNode;", "getBottom", "I", &[]);
    let get_width = graphics_method("Landroid/graphics/RenderNode;", "getWidth", "I", &[]);
    let get_height = graphics_method("Landroid/graphics/RenderNode;", "getHeight", "I", &[]);
    let get_unique_id = graphics_method("Landroid/graphics/RenderNode;", "getUniqueId", "J", &[]);
    let get_clip_to_bounds =
        graphics_method("Landroid/graphics/RenderNode;", "getClipToBounds", "Z", &[]);
    let set_clip_to_bounds = graphics_method(
        "Landroid/graphics/RenderNode;",
        "setClipToBounds",
        "Z",
        &["Z"],
    );
    let get_translation_x =
        graphics_method("Landroid/graphics/RenderNode;", "getTranslationX", "F", &[]);
    let set_translation_x = graphics_method(
        "Landroid/graphics/RenderNode;",
        "setTranslationX",
        "Z",
        &["F"],
    );
    let get_rotation_z = graphics_method("Landroid/graphics/RenderNode;", "getRotationZ", "F", &[]);
    let set_rotation_z =
        graphics_method("Landroid/graphics/RenderNode;", "setRotationZ", "Z", &["F"]);
    let get_ambient_shadow_color = graphics_method(
        "Landroid/graphics/RenderNode;",
        "getAmbientShadowColor",
        "I",
        &[],
    );
    let set_ambient_shadow_color = graphics_method(
        "Landroid/graphics/RenderNode;",
        "setAmbientShadowColor",
        "Z",
        &["I"],
    );
    let begin_recording = graphics_method(
        "Landroid/graphics/RenderNode;",
        "beginRecording",
        "Landroid/graphics/RecordingCanvas;",
        &["I", "I"],
    );
    let end_recording = graphics_method("Landroid/graphics/RenderNode;", "endRecording", "V", &[]);
    let has_display_list =
        graphics_method("Landroid/graphics/RenderNode;", "hasDisplayList", "Z", &[]);
    let get_clip_bounds = graphics_method(
        "Landroid/graphics/Canvas;",
        "getClipBounds",
        "Z",
        &["Landroid/graphics/Rect;"],
    );
    let draw_rect = graphics_method(
        "Landroid/graphics/Canvas;",
        "drawRect",
        "V",
        &["F", "F", "F", "F", "Landroid/graphics/Paint;"],
    );
    let draw_render_node = graphics_method(
        "Landroid/graphics/Canvas;",
        "drawRenderNode",
        "V",
        &["Landroid/graphics/RenderNode;"],
    );
    let render_node = DexValue::Object(DexObjectRef {
        id: 80,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/RenderNode;"),
    });
    let paint = DexValue::Object(DexObjectRef {
        id: 81,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Paint;"),
    });
    let rect = DexValue::Object(DexObjectRef {
        id: 82,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Rect;"),
    });
    let target_canvas = DexValue::Object(DexObjectRef {
        id: 83,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Canvas;"),
    });

    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![
                render_node.clone(),
                DexValue::String(std::rc::Rc::from("Compose"))
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_clip_to_bounds,
            Some(&test_invocation(vec![render_node.clone()]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_clip_to_bounds,
            Some(&test_invocation(vec![
                render_node.clone(),
                DexValue::Int(0)
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_clip_to_bounds,
            Some(&test_invocation(vec![render_node.clone()]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_position,
            Some(&test_invocation(vec![
                render_node.clone(),
                DexValue::Int(10),
                DexValue::Int(20),
                DexValue::Int(110),
                DexValue::Int(70)
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&get_left, Some(&test_invocation(vec![render_node.clone()]))),
        Some(DexExecutionResult::Int(10))
    );
    assert_eq!(
        program.invoke_builtin_method(&get_top, Some(&test_invocation(vec![render_node.clone()]))),
        Some(DexExecutionResult::Int(20))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_right,
            Some(&test_invocation(vec![render_node.clone()]))
        ),
        Some(DexExecutionResult::Int(110))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_bottom,
            Some(&test_invocation(vec![render_node.clone()]))
        ),
        Some(DexExecutionResult::Int(70))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_width,
            Some(&test_invocation(vec![render_node.clone()]))
        ),
        Some(DexExecutionResult::Int(100))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_height,
            Some(&test_invocation(vec![render_node.clone()]))
        ),
        Some(DexExecutionResult::Int(50))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &offset_left_and_right,
            Some(&test_invocation(vec![
                render_node.clone(),
                DexValue::Int(5)
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &offset_top_and_bottom,
            Some(&test_invocation(vec![
                render_node.clone(),
                DexValue::Int(-10)
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&get_left, Some(&test_invocation(vec![render_node.clone()]))),
        Some(DexExecutionResult::Int(15))
    );
    assert_eq!(
        program.invoke_builtin_method(&get_top, Some(&test_invocation(vec![render_node.clone()]))),
        Some(DexExecutionResult::Int(10))
    );
    let Some(DexExecutionResult::Value(first_unique_id)) = program.invoke_builtin_method(
        &get_unique_id,
        Some(&test_invocation(vec![render_node.clone()])),
    ) else {
        panic!("AOSP RenderNode.getUniqueId should return a long value");
    };
    assert_ne!(first_unique_id, DexValue::Long(0));
    assert_eq!(
        program.invoke_builtin_method(
            &get_unique_id,
            Some(&test_invocation(vec![render_node.clone()]))
        ),
        Some(DexExecutionResult::Value(first_unique_id))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_translation_x,
            Some(&test_invocation(vec![
                render_node.clone(),
                DexValue::Float(3.5)
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_translation_x,
            Some(&test_invocation(vec![render_node.clone()]))
        ),
        Some(DexExecutionResult::Value(DexValue::Float(3.5)))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_rotation_z,
            Some(&test_invocation(vec![
                render_node.clone(),
                DexValue::Float(45.0)
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_rotation_z,
            Some(&test_invocation(vec![render_node.clone()]))
        ),
        Some(DexExecutionResult::Value(DexValue::Float(45.0)))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_ambient_shadow_color,
            Some(&test_invocation(vec![
                render_node.clone(),
                DexValue::Int(0xff12_3456u32 as i32)
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_ambient_shadow_color,
            Some(&test_invocation(vec![render_node.clone()]))
        ),
        Some(DexExecutionResult::Int(0xff12_3456u32 as i32))
    );
    let Some(DexExecutionResult::Value(recording_canvas)) = program.invoke_builtin_method(
        &begin_recording,
        Some(&test_invocation(vec![
            render_node.clone(),
            DexValue::Int(32),
            DexValue::Int(24),
        ])),
    ) else {
        panic!("AOSP RenderNode.beginRecording(width,height) should return a RecordingCanvas");
    };
    assert_eq!(
        program.invoke_builtin_method(
            &get_clip_bounds,
            Some(&test_invocation(vec![
                recording_canvas.clone(),
                rect.clone()
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.read_stored_instance_field(&rect, &rect_int_field("right")),
        Some(DexValue::Int(32))
    );
    assert_eq!(
        program.read_stored_instance_field(&rect, &rect_int_field("bottom")),
        Some(DexValue::Int(24))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &draw_rect,
            Some(&test_invocation(vec![
                recording_canvas.clone(),
                DexValue::Float(1.0),
                DexValue::Float(2.0),
                DexValue::Float(3.0),
                DexValue::Float(4.0),
                paint.clone()
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &end_recording,
            Some(&test_invocation(vec![render_node.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &has_display_list,
            Some(&test_invocation(vec![render_node.clone()]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &draw_render_node,
            Some(&test_invocation(vec![
                target_canvas.clone(),
                render_node.clone()
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .canvas_draw_ops
            .get(&map_identity_key(&target_canvas)),
        Some(&vec![CanvasDrawOp::RenderNode {
            render_node: render_node.clone()
        }])
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .render_node_draw_ops
            .get(&map_identity_key(&render_node)),
        Some(&vec![CanvasDrawOp::Rect {
            left: 1.0,
            top: 2.0,
            right: 3.0,
            bottom: 4.0,
            paint
        }])
    );
}

#[test]
fn androidx_graphics_layer_update_display_list_records_only_dirty_render_node() {
    let program = DexProgram::new(Vec::new());
    let begin_recording = graphics_method(
        "Landroid/graphics/RenderNode;",
        "beginRecording",
        "Landroid/graphics/RecordingCanvas;",
        &["I", "I"],
    );
    let end_recording = graphics_method("Landroid/graphics/RenderNode;", "endRecording", "V", &[]);
    let has_display_list =
        graphics_method("Landroid/graphics/RenderNode;", "hasDisplayList", "Z", &[]);
    let discard_display_list = graphics_method(
        "Landroid/graphics/RenderNode;",
        "discardDisplayList",
        "V",
        &[],
    );
    let set_translation_x = graphics_method(
        "Landroid/graphics/RenderNode;",
        "setTranslationX",
        "Z",
        &["F"],
    );
    let draw_rect = graphics_method(
        "Landroid/graphics/Canvas;",
        "drawRect",
        "V",
        &["F", "F", "F", "F", "Landroid/graphics/Paint;"],
    );
    let draw_render_node = graphics_method(
        "Landroid/graphics/Canvas;",
        "drawRenderNode",
        "V",
        &["Landroid/graphics/RenderNode;"],
    );
    let first = DexValue::Object(DexObjectRef {
        id: 91,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/RenderNode;"),
    });
    let second = DexValue::Object(DexObjectRef {
        id: 92,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/RenderNode;"),
    });
    let paint = DexValue::Object(DexObjectRef {
        id: 93,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Paint;"),
    });
    let target_canvas = DexValue::Object(DexObjectRef {
        id: 94,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Canvas;"),
    });

    let Some(DexExecutionResult::Value(first_canvas)) = program.invoke_builtin_method(
        &begin_recording,
        Some(&test_invocation(vec![
            first.clone(),
            DexValue::Int(40),
            DexValue::Int(40),
        ])),
    ) else {
        panic!("AndroidX GraphicsLayerV29.record should begin first layer recording");
    };
    assert_eq!(
        program.invoke_builtin_method(
            &draw_rect,
            Some(&test_invocation(vec![
                first_canvas,
                DexValue::Float(1.0),
                DexValue::Float(2.0),
                DexValue::Float(3.0),
                DexValue::Float(4.0),
                paint.clone(),
            ])),
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&end_recording, Some(&test_invocation(vec![first.clone()]))),
        Some(DexExecutionResult::Void)
    );

    let Some(DexExecutionResult::Value(second_canvas)) = program.invoke_builtin_method(
        &begin_recording,
        Some(&test_invocation(vec![
            second.clone(),
            DexValue::Int(40),
            DexValue::Int(40),
        ])),
    ) else {
        panic!("AndroidX GraphicsLayerV29.record should begin second layer recording");
    };
    assert_eq!(
        program.invoke_builtin_method(
            &draw_rect,
            Some(&test_invocation(vec![
                second_canvas,
                DexValue::Float(10.0),
                DexValue::Float(20.0),
                DexValue::Float(30.0),
                DexValue::Float(40.0),
                paint.clone(),
            ])),
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&end_recording, Some(&test_invocation(vec![second.clone()]))),
        Some(DexExecutionResult::Void)
    );

    let first_key = map_identity_key(&first);
    let second_key = map_identity_key(&second);
    assert_eq!(
        program
            .runtime
            .borrow()
            .render_node_generations
            .get(&first_key),
        Some(&1)
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .render_node_generations
            .get(&second_key),
        Some(&1)
    );

    assert_eq!(
        program.invoke_builtin_method(
            &draw_render_node,
            Some(&test_invocation(vec![target_canvas.clone(), first.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &draw_render_node,
            Some(&test_invocation(vec![
                target_canvas.clone(),
                second.clone()
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .canvas_draw_ops
            .get(&map_identity_key(&target_canvas)),
        Some(&vec![
            CanvasDrawOp::RenderNode {
                render_node: first.clone(),
            },
            CanvasDrawOp::RenderNode {
                render_node: second.clone(),
            },
        ])
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .render_node_generations
            .get(&first_key),
        Some(&1)
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .render_node_generations
            .get(&second_key),
        Some(&1)
    );

    assert_eq!(
        program.invoke_builtin_method(
            &set_translation_x,
            Some(&test_invocation(vec![first.clone(), DexValue::Float(8.0)]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .render_node_generations
            .get(&first_key),
        Some(&1)
    );

    let Some(DexExecutionResult::Value(first_redraw_canvas)) = program.invoke_builtin_method(
        &begin_recording,
        Some(&test_invocation(vec![
            first.clone(),
            DexValue::Int(40),
            DexValue::Int(40),
        ])),
    ) else {
        panic!("AndroidX GraphicsLayerOwnerLayer.updateDisplayList should rerecord dirty layer");
    };
    assert_eq!(
        program.invoke_builtin_method(
            &draw_rect,
            Some(&test_invocation(vec![
                first_redraw_canvas,
                DexValue::Float(5.0),
                DexValue::Float(6.0),
                DexValue::Float(7.0),
                DexValue::Float(8.0),
                paint.clone(),
            ])),
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&end_recording, Some(&test_invocation(vec![first.clone()]))),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .render_node_generations
            .get(&first_key),
        Some(&2)
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .render_node_generations
            .get(&second_key),
        Some(&1)
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .render_node_draw_ops
            .get(&first_key),
        Some(&vec![CanvasDrawOp::Rect {
            left: 5.0,
            top: 6.0,
            right: 7.0,
            bottom: 8.0,
            paint: paint.clone(),
        }])
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .render_node_draw_ops
            .get(&second_key),
        Some(&vec![CanvasDrawOp::Rect {
            left: 10.0,
            top: 20.0,
            right: 30.0,
            bottom: 40.0,
            paint,
        }])
    );
    assert_eq!(
        program.invoke_builtin_method(
            &discard_display_list,
            Some(&test_invocation(vec![second.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&has_display_list, Some(&test_invocation(vec![second]))),
        Some(DexExecutionResult::Int(0))
    );
    assert!(!program
        .runtime
        .borrow()
        .render_node_generations
        .contains_key(&second_key));
}

#[test]
fn builtin_render_node_constructor_stores_name() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Landroid/graphics/RenderNode;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned()],
        },
    };
    let set_clip_to_bounds = DexMethodId {
        class_descriptor: "Landroid/graphics/RenderNode;".to_owned(),
        name: "setClipToBounds".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZZ".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Z".to_owned()],
        },
    };
    let begin_recording = DexMethodId {
        class_descriptor: "Landroid/graphics/RenderNode;".to_owned(),
        name: "beginRecording".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/graphics/RecordingCanvas;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let end_recording = DexMethodId {
        class_descriptor: "Landroid/graphics/RenderNode;".to_owned(),
        name: "endRecording".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let has_display_list = DexMethodId {
        class_descriptor: "Landroid/graphics/RenderNode;".to_owned(),
        name: "hasDisplayList".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let discard_display_list = DexMethodId {
        class_descriptor: "Landroid/graphics/RenderNode;".to_owned(),
        name: "discardDisplayList".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get_matrix = DexMethodId {
        class_descriptor: "Landroid/graphics/RenderNode;".to_owned(),
        name: "getMatrix".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/graphics/Matrix;".to_owned()],
        },
    };
    let is_hardware_accelerated = DexMethodId {
        class_descriptor: "Landroid/graphics/Canvas;".to_owned(),
        name: "isHardwareAccelerated".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set_use_compositing_layer = DexMethodId {
        class_descriptor: "Landroid/graphics/RenderNode;".to_owned(),
        name: "setUseCompositingLayer".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Z".to_owned(), "Landroid/graphics/Paint;".to_owned()],
        },
    };
    let set_clip_to_outline = DexMethodId {
        class_descriptor: "Landroid/graphics/RenderNode;".to_owned(),
        name: "setClipToOutline".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZZ".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Z".to_owned()],
        },
    };
    let set_elevation = DexMethodId {
        class_descriptor: "Landroid/graphics/RenderNode;".to_owned(),
        name: "setElevation".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZF".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["F".to_owned()],
        },
    };
    let set_alpha = DexMethodId {
        class_descriptor: "Landroid/graphics/RenderNode;".to_owned(),
        name: "setAlpha".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZF".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["F".to_owned()],
        },
    };
    let set_scale_x = DexMethodId {
        class_descriptor: "Landroid/graphics/RenderNode;".to_owned(),
        name: "setScaleX".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZF".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["F".to_owned()],
        },
    };
    let set_scale_y = DexMethodId {
        class_descriptor: "Landroid/graphics/RenderNode;".to_owned(),
        name: "setScaleY".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZF".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["F".to_owned()],
        },
    };
    let set_position = DexMethodId {
        class_descriptor: "Landroid/graphics/RenderNode;".to_owned(),
        name: "setPosition".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZIIII".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec![
                "I".to_owned(),
                "I".to_owned(),
                "I".to_owned(),
                "I".to_owned(),
            ],
        },
    };
    let set_pivot_x = DexMethodId {
        class_descriptor: "Landroid/graphics/RenderNode;".to_owned(),
        name: "setPivotX".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZF".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["F".to_owned()],
        },
    };
    let set_pivot_y = DexMethodId {
        class_descriptor: "Landroid/graphics/RenderNode;".to_owned(),
        name: "setPivotY".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZF".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["F".to_owned()],
        },
    };
    let set_has_overlapping_rendering = DexMethodId {
        class_descriptor: "Landroid/graphics/RenderNode;".to_owned(),
        name: "setHasOverlappingRendering".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZZ".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Z".to_owned()],
        },
    };
    let set_outline = DexMethodId {
        class_descriptor: "Landroid/graphics/RenderNode;".to_owned(),
        name: "setOutline".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["Landroid/graphics/Outline;".to_owned()],
        },
    };
    let render_node = DexValue::Object(DexObjectRef {
        id: 9,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/RenderNode;"),
    });
    let paint = DexValue::Object(DexObjectRef {
        id: 10,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Paint;"),
    });
    let outline = DexValue::Object(DexObjectRef {
        id: 11,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Outline;"),
    });
    let matrix = DexValue::Object(DexObjectRef {
        id: 12,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Matrix;"),
    });

    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![
                render_node.clone(),
                DexValue::String(std::rc::Rc::from("Compose"))
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&render_node, &render_node_name_field()),
        Some(DexValue::String(std::rc::Rc::from("Compose")))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &has_display_list,
            Some(&test_invocation(vec![render_node.clone()]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    let Some(DexExecutionResult::Value(recording_canvas)) = program.invoke_builtin_method(
        &begin_recording,
        Some(&test_invocation(vec![render_node.clone()])),
    ) else {
        panic!("beginRecording should return a RecordingCanvas");
    };
    assert_eq!(
        value_class_descriptor(&recording_canvas),
        Some("Landroid/graphics/RecordingCanvas;")
    );
    assert_eq!(
        program
            .read_stored_instance_field(&recording_canvas, &recording_canvas_render_node_field()),
        Some(render_node.clone())
    );
    assert_eq!(
        program.read_stored_instance_field(&render_node, &render_node_recording_canvas_field()),
        Some(recording_canvas.clone())
    );
    assert_eq!(
        program.invoke_builtin_method(
            &is_hardware_accelerated,
            Some(&test_invocation(vec![recording_canvas.clone()]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    let recorded_ops = vec![CanvasDrawOp::Rect {
        left: 1.0,
        top: 2.0,
        right: 3.0,
        bottom: 4.0,
        paint: paint.clone(),
    }];
    program
        .runtime
        .borrow_mut()
        .canvas_draw_ops
        .insert(map_identity_key(&recording_canvas), recorded_ops.clone());
    assert_eq!(
        program.invoke_builtin_method(
            &end_recording,
            Some(&test_invocation(vec![render_node.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .render_node_draw_ops
            .get(&map_identity_key(&render_node))
            .cloned(),
        Some(recorded_ops)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &has_display_list,
            Some(&test_invocation(vec![render_node.clone()]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &discard_display_list,
            Some(&test_invocation(vec![render_node.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert!(!program
        .runtime
        .borrow()
        .render_node_draw_ops
        .contains_key(&map_identity_key(&render_node)));
    assert_eq!(
        program.invoke_builtin_method(
            &has_display_list,
            Some(&test_invocation(vec![render_node.clone()]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_matrix,
            Some(&test_invocation(vec![render_node.clone(), matrix.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .matrix_values
            .get(&map_identity_key(&matrix)),
        Some(&AndroidMatrixState::identity())
    );
    assert_eq!(
        program.read_stored_instance_field(&render_node, &render_node_recording_canvas_field()),
        Some(DexValue::Null)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_clip_to_bounds,
            Some(&test_invocation(vec![
                render_node.clone(),
                DexValue::Int(0)
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_clip_to_bounds,
            Some(&test_invocation(vec![
                render_node.clone(),
                DexValue::Int(1)
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.read_stored_instance_field(&render_node, &render_node_clip_to_bounds_field()),
        Some(DexValue::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_clip_to_bounds,
            Some(&test_invocation(vec![
                render_node.clone(),
                DexValue::Int(1)
            ]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_use_compositing_layer,
            Some(&test_invocation(vec![
                render_node.clone(),
                DexValue::Int(0),
                DexValue::Int(0)
            ]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_use_compositing_layer,
            Some(&test_invocation(vec![
                render_node.clone(),
                DexValue::Int(1),
                paint.clone()
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program
            .read_stored_instance_field(&render_node, &render_node_use_compositing_layer_field()),
        Some(DexValue::Int(1))
    );
    assert_eq!(
        program
            .read_stored_instance_field(&render_node, &render_node_compositing_layer_paint_field()),
        Some(paint)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_clip_to_outline,
            Some(&test_invocation(vec![
                render_node.clone(),
                DexValue::Int(1)
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.read_stored_instance_field(&render_node, &render_node_clip_to_outline_field()),
        Some(DexValue::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_clip_to_outline,
            Some(&test_invocation(vec![
                render_node.clone(),
                DexValue::Int(1)
            ]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_elevation,
            Some(&test_invocation(vec![
                render_node.clone(),
                DexValue::Float(4.5)
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.read_stored_instance_field(&render_node, &render_node_elevation_field()),
        Some(DexValue::Float(4.5))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_elevation,
            Some(&test_invocation(vec![
                render_node.clone(),
                DexValue::Float(4.5)
            ]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_alpha,
            Some(&test_invocation(vec![
                render_node.clone(),
                DexValue::Float(1.0)
            ]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_alpha,
            Some(&test_invocation(vec![
                render_node.clone(),
                DexValue::Float(0.5)
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.read_stored_instance_field(&render_node, &render_node_alpha_field()),
        Some(DexValue::Float(0.5))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_alpha,
            Some(&test_invocation(vec![
                render_node.clone(),
                DexValue::Float(0.5)
            ]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_scale_x,
            Some(&test_invocation(vec![
                render_node.clone(),
                DexValue::Float(1.0)
            ]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_scale_x,
            Some(&test_invocation(vec![
                render_node.clone(),
                DexValue::Float(0.8)
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.read_stored_instance_field(&render_node, &render_node_scale_x_field()),
        Some(DexValue::Float(0.8))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_scale_y,
            Some(&test_invocation(vec![
                render_node.clone(),
                DexValue::Float(1.0)
            ]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_scale_y,
            Some(&test_invocation(vec![
                render_node.clone(),
                DexValue::Float(0.8)
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.read_stored_instance_field(&render_node, &render_node_scale_y_field()),
        Some(DexValue::Float(0.8))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_position,
            Some(&test_invocation(vec![
                render_node.clone(),
                DexValue::Int(1),
                DexValue::Int(2),
                DexValue::Int(3),
                DexValue::Int(4),
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program
            .read_stored_instance_field(&render_node, &render_node_position_field("positionRight")),
        Some(DexValue::Int(3))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_position,
            Some(&test_invocation(vec![
                render_node.clone(),
                DexValue::Int(1),
                DexValue::Int(2),
                DexValue::Int(3),
                DexValue::Int(4),
            ]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_pivot_x,
            Some(&test_invocation(vec![
                render_node.clone(),
                DexValue::Float(5.5)
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.read_stored_instance_field(&render_node, &render_node_pivot_x_field()),
        Some(DexValue::Float(5.5))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_pivot_x,
            Some(&test_invocation(vec![
                render_node.clone(),
                DexValue::Float(5.5)
            ]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_pivot_y,
            Some(&test_invocation(vec![
                render_node.clone(),
                DexValue::Float(6.5)
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.read_stored_instance_field(&render_node, &render_node_pivot_y_field()),
        Some(DexValue::Float(6.5))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_pivot_y,
            Some(&test_invocation(vec![
                render_node.clone(),
                DexValue::Float(6.5)
            ]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_has_overlapping_rendering,
            Some(&test_invocation(vec![
                render_node.clone(),
                DexValue::Int(1)
            ]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_has_overlapping_rendering,
            Some(&test_invocation(vec![
                render_node.clone(),
                DexValue::Int(0)
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.read_stored_instance_field(
            &render_node,
            &render_node_has_overlapping_rendering_field()
        ),
        Some(DexValue::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_outline,
            Some(&test_invocation(vec![render_node.clone(), outline.clone()]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.read_stored_instance_field(&render_node, &render_node_outline_field()),
        Some(outline)
    );
}

#[test]
fn builtin_path_no_arg_constructor_is_noop() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Landroid/graphics/Path;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set_fill_type = DexMethodId {
        class_descriptor: "Landroid/graphics/Path;".to_owned(),
        name: "setFillType".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/graphics/Path$FillType;".to_owned()],
        },
    };
    let get_fill_type = DexMethodId {
        class_descriptor: "Landroid/graphics/Path;".to_owned(),
        name: "getFillType".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/graphics/Path$FillType;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let rewind = DexMethodId {
        class_descriptor: "Landroid/graphics/Path;".to_owned(),
        name: "rewind".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let move_to = DexMethodId {
        class_descriptor: "Landroid/graphics/Path;".to_owned(),
        name: "moveTo".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VFF".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["F".to_owned(), "F".to_owned()],
        },
    };
    let line_to = DexMethodId {
        class_descriptor: "Landroid/graphics/Path;".to_owned(),
        name: "lineTo".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VFF".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["F".to_owned(), "F".to_owned()],
        },
    };
    let r_line_to = DexMethodId {
        class_descriptor: "Landroid/graphics/Path;".to_owned(),
        name: "rLineTo".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VFF".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["F".to_owned(), "F".to_owned()],
        },
    };
    let quad_to = DexMethodId {
        class_descriptor: "Landroid/graphics/Path;".to_owned(),
        name: "quadTo".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VFFFF".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "F".to_owned(),
                "F".to_owned(),
                "F".to_owned(),
                "F".to_owned(),
            ],
        },
    };
    let add_round_rect = DexMethodId {
        class_descriptor: "Landroid/graphics/Path;".to_owned(),
        name: "addRoundRect".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VLLL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "Landroid/graphics/RectF;".to_owned(),
                "[F".to_owned(),
                "Landroid/graphics/Path$Direction;".to_owned(),
            ],
        },
    };
    let add_rect = DexMethodId {
        class_descriptor: "Landroid/graphics/Path;".to_owned(),
        name: "addRect".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VLL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "Landroid/graphics/RectF;".to_owned(),
                "Landroid/graphics/Path$Direction;".to_owned(),
            ],
        },
    };
    let close = DexMethodId {
        class_descriptor: "Landroid/graphics/Path;".to_owned(),
        name: "close".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let path = DexValue::Object(DexObjectRef {
        id: 1,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Path;"),
    });
    let rectf = DexValue::Object(DexObjectRef {
        id: 2,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/RectF;"),
    });
    for (name, value) in [
        ("left", 1.0),
        ("top", 2.0),
        ("right", 31.0),
        ("bottom", 42.0),
    ] {
        assert!(program.write_stored_instance_field(
            &rectf,
            &rectf_float_field(name),
            DexValue::Float(value),
        ));
    }
    let radii = program
        .allocate_array("[F".to_owned(), 8)
        .expect("radii array should allocate");
    for index in 0..8 {
        program
            .write_array_value(&radii, index, DexValue::Float(index as f32 + 0.5))
            .expect("radii array write should succeed");
    }
    let direction = DexValue::Reference(std::rc::Rc::from("Landroid/graphics/Path$Direction;.CW"));
    let default_fill_type = DexValue::Reference(std::rc::Rc::from(
        "Landroid/graphics/Path$FillType;.WINDING",
    ));
    let fill_type = DexValue::Reference(std::rc::Rc::from(
        "Landroid/graphics/Path$FillType;.EVEN_ODD",
    ));

    assert_eq!(
        program.invoke_builtin_method(&constructor, Some(&test_invocation(vec![path.clone()]))),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&get_fill_type, Some(&test_invocation(vec![path.clone()]))),
        Some(DexExecutionResult::Value(default_fill_type))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_fill_type,
            Some(&test_invocation(vec![path.clone(), fill_type.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&path, &path_fill_type_field()),
        Some(fill_type.clone())
    );
    assert_eq!(
        program.invoke_builtin_method(&get_fill_type, Some(&test_invocation(vec![path.clone()]))),
        Some(DexExecutionResult::Value(fill_type))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &move_to,
            Some(&test_invocation(vec![
                path.clone(),
                DexValue::Float(12.0),
                DexValue::Float(24.0),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &line_to,
            Some(&test_invocation(vec![
                path.clone(),
                DexValue::Float(36.0),
                DexValue::Float(48.0),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &r_line_to,
            Some(&test_invocation(vec![
                path.clone(),
                DexValue::Float(5.0),
                DexValue::Float(6.0),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &quad_to,
            Some(&test_invocation(vec![
                path.clone(),
                DexValue::Float(1.0),
                DexValue::Float(2.0),
                DexValue::Float(3.0),
                DexValue::Float(4.0),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &add_rect,
            Some(&test_invocation(vec![
                path.clone(),
                rectf.clone(),
                direction.clone(),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &add_round_rect,
            Some(&test_invocation(vec![
                path.clone(),
                rectf.clone(),
                radii.clone(),
                direction.clone(),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&close, Some(&test_invocation(vec![path.clone()]))),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .path_ops
            .get(&map_identity_key(&path)),
        Some(&vec![
            PathOp::MoveTo { x: 12.0, y: 24.0 },
            PathOp::LineTo { x: 36.0, y: 48.0 },
            PathOp::RLineTo { dx: 5.0, dy: 6.0 },
            PathOp::QuadTo {
                x1: 1.0,
                y1: 2.0,
                x2: 3.0,
                y2: 4.0,
            },
            PathOp::Rect {
                left: 1.0,
                top: 2.0,
                right: 31.0,
                bottom: 42.0,
                clockwise: true,
            },
            PathOp::RoundRect {
                left: 1.0,
                top: 2.0,
                right: 31.0,
                bottom: 42.0,
                radii: vec![0.5, 1.5, 2.5, 3.5, 4.5, 5.5, 6.5, 7.5],
                clockwise: true,
            },
            PathOp::Close,
        ])
    );
    assert_eq!(
        program.invoke_builtin_method(&rewind, Some(&test_invocation(vec![path.clone()]))),
        Some(DexExecutionResult::Void)
    );
    assert!(!program
        .runtime
        .borrow()
        .path_ops
        .contains_key(&map_identity_key(&path)));
    assert_eq!(
        program.read_stored_instance_field(&path, &path_fill_type_field()),
        Some(DexValue::Reference(std::rc::Rc::from(
            "Landroid/graphics/Path$FillType;.EVEN_ODD"
        )))
    );
}

#[test]
fn builtin_path_op_clones_source_ops_into_receiver() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/graphics/Path;".to_owned(),
        name: "op".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZLLL".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec![
                "Landroid/graphics/Path;".to_owned(),
                "Landroid/graphics/Path;".to_owned(),
                "Landroid/graphics/Path$Op;".to_owned(),
            ],
        },
    };
    let receiver = DexValue::Object(DexObjectRef {
        id: 10,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Path;"),
    });
    let left = DexValue::Object(DexObjectRef {
        id: 11,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Path;"),
    });
    let right = DexValue::Object(DexObjectRef {
        id: 12,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Path;"),
    });
    let op = DexValue::Reference(std::rc::Rc::from("Landroid/graphics/Path$Op;.INTERSECT"));

    {
        let mut runtime = program.runtime.borrow_mut();
        runtime.path_ops.insert(
            map_identity_key(&left),
            vec![PathOp::MoveTo { x: 1.0, y: 2.0 }],
        );
        runtime.path_ops.insert(
            map_identity_key(&right),
            vec![PathOp::LineTo { x: 3.0, y: 4.0 }],
        );
    }

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                receiver.clone(),
                left.clone(),
                right.clone(),
                op,
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );

    program
        .runtime
        .borrow_mut()
        .path_ops
        .get_mut(&map_identity_key(&left))
        .expect("left path ops should remain available")
        .push(PathOp::LineTo { x: 9.0, y: 10.0 });

    assert_eq!(
        program
            .runtime
            .borrow()
            .path_ops
            .get(&map_identity_key(&receiver)),
        Some(&vec![PathOp::BooleanOp {
            left: vec![PathOp::MoveTo { x: 1.0, y: 2.0 }],
            right: vec![PathOp::LineTo { x: 3.0, y: 4.0 }],
            op: PathBooleanOp::Intersect,
        }])
    );
}

#[test]
fn builtin_blend_mode_color_filter_constructor_stores_color_and_mode() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Landroid/graphics/BlendModeColorFilter;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VIL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned(), "Landroid/graphics/BlendMode;".to_owned()],
        },
    };
    let filter = DexValue::Object(DexObjectRef {
        id: 2,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/BlendModeColorFilter;"),
    });
    let mode = DexValue::Reference(std::rc::Rc::from("Landroid/graphics/BlendMode;.SRC_IN"));

    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![
                filter.clone(),
                DexValue::Int(0xff00_ff00u32 as i32),
                mode.clone()
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&filter, &blend_mode_color_filter_color_field()),
        Some(DexValue::Int(0xff00_ff00u32 as i32))
    );
    assert_eq!(
        program.read_stored_instance_field(&filter, &blend_mode_color_filter_mode_field()),
        Some(mode)
    );
}

#[test]
fn builtin_matrix_no_arg_constructor_stores_identity_state() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Landroid/graphics/Matrix;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let translate = DexMethodId {
        class_descriptor: "Landroid/graphics/Matrix;".to_owned(),
        name: "preTranslate".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZFF".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["F".to_owned(), "F".to_owned()],
        },
    };
    let post_translate = DexMethodId {
        name: "postTranslate".to_owned(),
        ..translate.clone()
    };
    let reset = DexMethodId {
        class_descriptor: "Landroid/graphics/Matrix;".to_owned(),
        name: "reset".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get_values = DexMethodId {
        class_descriptor: "Landroid/graphics/Matrix;".to_owned(),
        name: "getValues".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V[".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["[F".to_owned()],
        },
    };
    let matrix = DexValue::Object(DexObjectRef {
        id: 1,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Matrix;"),
    });
    let values = program
        .allocate_array("[F".to_owned(), 9)
        .expect("float array should allocate");

    assert_eq!(
        program.invoke_builtin_method(&constructor, Some(&test_invocation(vec![matrix.clone()]))),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .matrix_values
            .get(&map_identity_key(&matrix)),
        Some(&AndroidMatrixState::identity())
    );
    assert_eq!(
        program.invoke_builtin_method(
            &translate,
            Some(&test_invocation(vec![
                matrix.clone(),
                DexValue::Float(7.0),
                DexValue::Float(11.0),
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    let mut translated = AndroidMatrixState::identity();
    translated.values[2] = 7.0;
    translated.values[5] = 11.0;
    assert_eq!(
        program
            .runtime
            .borrow()
            .matrix_values
            .get(&map_identity_key(&matrix)),
        Some(&translated)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &post_translate,
            Some(&test_invocation(vec![
                matrix.clone(),
                DexValue::Float(2.0),
                DexValue::Float(3.0),
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    translated.values[2] = 9.0;
    translated.values[5] = 14.0;
    assert_eq!(
        program
            .runtime
            .borrow()
            .matrix_values
            .get(&map_identity_key(&matrix)),
        Some(&translated)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_values,
            Some(&test_invocation(vec![matrix.clone(), values.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .array_entries
            .get(&map_identity_key(&values)),
        Some(&translated.values.map(DexValue::Float).to_vec())
    );
    assert_eq!(
        program.invoke_builtin_method(&reset, Some(&test_invocation(vec![matrix.clone()]))),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .matrix_values
            .get(&map_identity_key(&matrix)),
        Some(&AndroidMatrixState::identity())
    );
}

#[test]
fn builtin_view_transform_matrix_translates_by_view_global_offset() {
    let program = DexProgram::new(Vec::new());
    let transform_global = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "transformMatrixToGlobal".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/graphics/Matrix;".to_owned()],
        },
    };
    let transform_local = DexMethodId {
        class_descriptor: "Landroid/view/View;".to_owned(),
        name: "transformMatrixToLocal".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/graphics/Matrix;".to_owned()],
        },
    };
    let parent = DexValue::Reference(std::rc::Rc::from("Landroid/view/ViewGroup;:parent"));
    let child = DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:child"));
    let matrix = DexValue::Object(DexObjectRef {
        id: 7,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Matrix;"),
    });

    for (name, value) in [("left", 10), ("top", 20), ("right", 210), ("bottom", 220)] {
        assert!(program.write_stored_instance_field(
            &parent,
            &view_bounds_int_field(name),
            DexValue::Int(value),
        ));
    }
    for (name, value) in [("left", 3), ("top", 4), ("right", 103), ("bottom", 104)] {
        assert!(program.write_stored_instance_field(
            &child,
            &view_bounds_int_field(name),
            DexValue::Int(value),
        ));
    }
    program
        .runtime
        .borrow_mut()
        .view_parents
        .insert(map_identity_key(&child), parent);

    assert_eq!(
        program.invoke_builtin_method(
            &transform_global,
            Some(&test_invocation(vec![child.clone(), matrix.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    let mut translated = AndroidMatrixState::identity();
    translated.values[2] = 13.0;
    translated.values[5] = 24.0;
    assert_eq!(
        program
            .runtime
            .borrow()
            .matrix_values
            .get(&map_identity_key(&matrix)),
        Some(&translated)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &transform_local,
            Some(&test_invocation(vec![child, matrix.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .matrix_values
            .get(&map_identity_key(&matrix)),
        Some(&AndroidMatrixState::identity())
    );
}

#[test]
fn builtin_rectf_no_arg_constructor_stores_zero_edges() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Landroid/graphics/RectF;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set = DexMethodId {
        class_descriptor: "Landroid/graphics/RectF;".to_owned(),
        name: "set".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VFFFF".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "F".to_owned(),
                "F".to_owned(),
                "F".to_owned(),
                "F".to_owned(),
            ],
        },
    };
    let rect = DexValue::Object(DexObjectRef {
        id: 19,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/RectF;"),
    });

    assert_eq!(
        program.invoke_builtin_method(&constructor, Some(&test_invocation(vec![rect.clone()]))),
        Some(DexExecutionResult::Void)
    );
    for name in ["left", "top", "right", "bottom"] {
        assert_eq!(
            program.read_stored_instance_field(&rect, &rectf_float_field(name)),
            Some(DexValue::Float(0.0))
        );
    }
    assert_eq!(
        program.invoke_builtin_method(
            &set,
            Some(&test_invocation(vec![
                rect.clone(),
                DexValue::Float(1.5),
                DexValue::Float(2.5),
                DexValue::Float(3.5),
                DexValue::Float(4.5),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    for (name, value) in [("left", 1.5), ("top", 2.5), ("right", 3.5), ("bottom", 4.5)] {
        assert_eq!(
            program.read_stored_instance_field(&rect, &rectf_float_field(name)),
            Some(DexValue::Float(value))
        );
    }
}

#[test]
fn builtin_rect_set_stores_edges() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Landroid/graphics/Rect;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let edge_constructor = DexMethodId {
        class_descriptor: "Landroid/graphics/Rect;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VIIII".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "I".to_owned(),
                "I".to_owned(),
                "I".to_owned(),
                "I".to_owned(),
            ],
        },
    };
    let set = DexMethodId {
        class_descriptor: "Landroid/graphics/Rect;".to_owned(),
        name: "set".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VIIII".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "I".to_owned(),
                "I".to_owned(),
                "I".to_owned(),
                "I".to_owned(),
            ],
        },
    };
    let set_rect = DexMethodId {
        class_descriptor: "Landroid/graphics/Rect;".to_owned(),
        name: "set".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/graphics/Rect;".to_owned()],
        },
    };
    let set_empty = DexMethodId {
        class_descriptor: "Landroid/graphics/Rect;".to_owned(),
        name: "setEmpty".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get_text_bounds = DexMethodId {
        class_descriptor: "Landroid/graphics/Paint;".to_owned(),
        name: "getTextBounds".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VLIIL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "Ljava/lang/String;".to_owned(),
                "I".to_owned(),
                "I".to_owned(),
                "Landroid/graphics/Rect;".to_owned(),
            ],
        },
    };
    let get_char_sequence_bounds = DexMethodId {
        class_descriptor: "Landroid/graphics/Paint;".to_owned(),
        name: "getTextBounds".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VLIIL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "Ljava/lang/CharSequence;".to_owned(),
                "I".to_owned(),
                "I".to_owned(),
                "Landroid/graphics/Rect;".to_owned(),
            ],
        },
    };
    let width = DexMethodId {
        class_descriptor: "Landroid/graphics/Rect;".to_owned(),
        name: "width".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let height = DexMethodId {
        class_descriptor: "Landroid/graphics/Rect;".to_owned(),
        name: "height".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let rect = DexValue::Reference(std::rc::Rc::from("Landroid/graphics/Rect;:content"));
    let source_rect = DexValue::Reference(std::rc::Rc::from("Landroid/graphics/Rect;:source"));
    let paint = DexValue::Reference(std::rc::Rc::from("Landroid/text/TextPaint;:content"));
    let zero_size_view = DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:id/1"));
    let zero_size_paint = DexValue::Reference(std::rc::Rc::from(format!(
        "Landroid/text/TextPaint;:{}",
        map_identity_key(&zero_size_view)
    )));
    let spannable = DexValue::Object(DexObjectRef {
        id: 18,
        class_descriptor: std::rc::Rc::from("Landroid/text/SpannableString;"),
    });
    let invocation = test_invocation(vec![
        rect.clone(),
        DexValue::Int(1),
        DexValue::Int(2),
        DexValue::Int(3),
        DexValue::Int(4),
    ]);
    let source_invocation = test_invocation(vec![
        source_rect.clone(),
        DexValue::Int(10),
        DexValue::Int(20),
        DexValue::Int(30),
        DexValue::Int(40),
    ]);
    let text_bounds_invocation = test_invocation(vec![
        paint.clone(),
        DexValue::String(std::rc::Rc::from(String::new())),
        DexValue::Int(0),
        DexValue::Int(0),
        rect.clone(),
    ]);
    let sized_text_bounds_invocation = test_invocation(vec![
        paint.clone(),
        DexValue::String(std::rc::Rc::from("test")),
        DexValue::Int(0),
        DexValue::Int(4),
        rect.clone(),
    ]);
    let nonempty_text_bounds_invocation = test_invocation(vec![
        zero_size_paint.clone(),
        DexValue::String(std::rc::Rc::from("7")),
        DexValue::Int(0),
        DexValue::Int(1),
        rect.clone(),
    ]);
    program
        .runtime
        .borrow_mut()
        .string_values
        .insert(map_identity_key(&spannable), "7".to_owned());
    let char_sequence_bounds_invocation = test_invocation(vec![
        zero_size_paint,
        spannable,
        DexValue::Int(0),
        DexValue::Int(1),
        rect.clone(),
    ]);

    assert_eq!(
        program.invoke_builtin_method(&constructor, Some(&test_invocation(vec![rect.clone()]))),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&rect, &rect_int_field("left")),
        Some(DexValue::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&edge_constructor, Some(&invocation)),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&rect, &rect_int_field("left")),
        Some(DexValue::Int(1))
    );
    assert_eq!(
        program.read_stored_instance_field(&rect, &rect_int_field("bottom")),
        Some(DexValue::Int(4))
    );
    assert_eq!(
        program.invoke_builtin_method(&set, Some(&invocation)),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&rect, &rect_int_field("left")),
        Some(DexValue::Int(1))
    );
    assert_eq!(
        program.read_stored_instance_field(&rect, &rect_int_field("top")),
        Some(DexValue::Int(2))
    );
    assert_eq!(
        program.read_stored_instance_field(&rect, &rect_int_field("right")),
        Some(DexValue::Int(3))
    );
    assert_eq!(
        program.read_stored_instance_field(&rect, &rect_int_field("bottom")),
        Some(DexValue::Int(4))
    );
    assert_eq!(
        program.invoke_builtin_method(&width, Some(&test_invocation(vec![rect.clone()]))),
        Some(DexExecutionResult::Int(2))
    );
    assert_eq!(
        program.invoke_builtin_method(&height, Some(&test_invocation(vec![rect.clone()]))),
        Some(DexExecutionResult::Int(2))
    );
    assert_eq!(
        program.invoke_builtin_method(&edge_constructor, Some(&source_invocation)),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_rect,
            Some(&test_invocation(vec![rect.clone(), source_rect.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&rect, &rect_int_field("left")),
        Some(DexValue::Int(10))
    );
    assert_eq!(
        program.read_stored_instance_field(&rect, &rect_int_field("bottom")),
        Some(DexValue::Int(40))
    );
    assert_eq!(
        program.invoke_builtin_method(&set_empty, Some(&test_invocation(vec![rect.clone()]))),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&rect, &rect_int_field("left")),
        Some(DexValue::Int(0))
    );
    assert_eq!(
        program.read_stored_instance_field(&rect, &rect_int_field("right")),
        Some(DexValue::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&get_text_bounds, Some(&text_bounds_invocation)),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&rect, &rect_int_field("left")),
        Some(DexValue::Int(0))
    );
    assert_eq!(
        program.read_stored_instance_field(&rect, &rect_int_field("bottom")),
        Some(DexValue::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&width, Some(&test_invocation(vec![rect.clone()]))),
        Some(DexExecutionResult::Int(0))
    );
    assert!(program.write_stored_instance_field(
        &paint,
        &paint_text_size_field(),
        DexValue::Float(20.0),
    ));
    assert_eq!(
        program.invoke_builtin_method(&get_text_bounds, Some(&sized_text_bounds_invocation)),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&rect, &rect_int_field("top")),
        Some(DexValue::Int(-20))
    );
    assert_eq!(
        program.invoke_builtin_method(&width, Some(&test_invocation(vec![rect.clone()]))),
        Some(DexExecutionResult::Int(40))
    );
    assert!(program.write_stored_instance_field(
        &zero_size_view,
        &text_view_text_size_field(),
        DexValue::Float(0.0),
    ));
    assert_eq!(
        program.invoke_builtin_method(&get_text_bounds, Some(&nonempty_text_bounds_invocation)),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&width, Some(&test_invocation(vec![rect.clone()]))),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_char_sequence_bounds,
            Some(&char_sequence_bounds_invocation)
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&width, Some(&test_invocation(vec![rect.clone()]))),
        Some(DexExecutionResult::Int(0))
    );
}

#[test]
fn builtin_drawable_set_bounds_copies_rect_edges() {
    let program = DexProgram::new(Vec::new());
    let set_rect = DexMethodId {
        class_descriptor: "Landroid/graphics/Rect;".to_owned(),
        name: "set".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VIIII".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "I".to_owned(),
                "I".to_owned(),
                "I".to_owned(),
                "I".to_owned(),
            ],
        },
    };
    let set_bounds = DexMethodId {
        class_descriptor: "Landroid/graphics/drawable/Drawable;".to_owned(),
        name: "setBounds".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/graphics/Rect;".to_owned()],
        },
    };
    let set_hotspot = DexMethodId {
        class_descriptor: "Landroid/graphics/drawable/Drawable;".to_owned(),
        name: "setHotspot".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VFF".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["F".to_owned(), "F".to_owned()],
        },
    };
    let set_state = DexMethodId {
        class_descriptor: "Landroid/graphics/drawable/Drawable;".to_owned(),
        name: "setState".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z[".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["[I".to_owned()],
        },
    };
    let rect = DexValue::Reference(std::rc::Rc::from("Landroid/graphics/Rect;:drawable-bounds"));
    let drawable = DexValue::Object(DexObjectRef {
        id: 91,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/drawable/RippleDrawable;"),
    });
    let state = program
        .allocate_array("[I".to_owned(), 2)
        .expect("state array should allocate");
    assert_eq!(
        program.write_array_value(&state, 0, DexValue::Int(16842919)),
        Some(())
    );
    assert_eq!(
        program.write_array_value(&state, 1, DexValue::Int(16842910)),
        Some(())
    );

    assert_eq!(
        program.invoke_builtin_method(
            &set_rect,
            Some(&test_invocation(vec![
                rect.clone(),
                DexValue::Int(4),
                DexValue::Int(5),
                DexValue::Int(40),
                DexValue::Int(50),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_bounds,
            Some(&test_invocation(vec![drawable.clone(), rect]))
        ),
        Some(DexExecutionResult::Void)
    );
    for (name, value) in [("left", 4), ("top", 5), ("right", 40), ("bottom", 50)] {
        assert_eq!(
            program.read_stored_instance_field(&drawable, &drawable_bounds_int_field(name)),
            Some(DexValue::Int(value))
        );
    }
    assert_eq!(
        program.invoke_builtin_method(
            &set_hotspot,
            Some(&test_invocation(vec![
                drawable.clone(),
                DexValue::Float(8.5),
                DexValue::Float(9.5)
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&drawable, &drawable_hotspot_float_field("x")),
        Some(DexValue::Float(8.5))
    );
    assert_eq!(
        program.read_stored_instance_field(&drawable, &drawable_hotspot_float_field("y")),
        Some(DexValue::Float(9.5))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_state,
            Some(&test_invocation(vec![drawable.clone(), state.clone()]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    let stored_state = program
        .read_stored_instance_field(&drawable, &drawable_state_field())
        .expect("drawable state should be stored");
    assert_eq!(
        program.invoke_builtin_method(
            &set_state,
            Some(&test_invocation(vec![drawable.clone(), state.clone()]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.write_array_value(&state, 0, DexValue::Int(0)),
        Some(())
    );
    assert_eq!(
        program.read_array_value(&stored_state, 0),
        Some(DexValue::Int(16842919))
    );
}

// cite: frameworks-base Path.java#isEmpty, computeBounds, cubicTo
#[test]
fn path_is_empty_and_compute_bounds_track_recorded_ops() {
    let program = DexProgram::new(Vec::new());
    let path = program.allocate_object("Landroid/graphics/Path;".to_owned());
    let bounds = program.allocate_object("Landroid/graphics/RectF;".to_owned());
    let build = |name: &str, parameters: Vec<String>, return_type: &str| DexMethodId {
        class_descriptor: "Landroid/graphics/Path;".to_owned(),
        name: name.to_owned(),
        proto: crate::dex::DexProto {
            shorty: String::new(),
            return_type: return_type.to_owned(),
            parameters,
        },
    };
    let is_empty = build("isEmpty", Vec::new(), "Z");

    assert_eq!(
        program.invoke_builtin_method(&is_empty, Some(&test_invocation(vec![path.clone()]))),
        Some(DexExecutionResult::Int(1))
    );

    let move_to = build("moveTo", vec!["F".to_owned(), "F".to_owned()], "V");
    program.invoke_builtin_method(
        &move_to,
        Some(&test_invocation(vec![
            path.clone(),
            DexValue::Float(10.0),
            DexValue::Float(20.0),
        ])),
    );
    let cubic_to = build("cubicTo", vec!["F".to_owned(); 6], "V");
    program.invoke_builtin_method(
        &cubic_to,
        Some(&test_invocation(vec![
            path.clone(),
            DexValue::Float(30.0),
            DexValue::Float(5.0),
            DexValue::Float(50.0),
            DexValue::Float(40.0),
            DexValue::Float(60.0),
            DexValue::Float(25.0),
        ])),
    );

    assert_eq!(
        program.invoke_builtin_method(&is_empty, Some(&test_invocation(vec![path.clone()]))),
        Some(DexExecutionResult::Int(0))
    );

    let compute_bounds = build(
        "computeBounds",
        vec!["Landroid/graphics/RectF;".to_owned(), "Z".to_owned()],
        "V",
    );
    assert_eq!(
        program.invoke_builtin_method(
            &compute_bounds,
            Some(&test_invocation(vec![
                path.clone(),
                bounds.clone(),
                DexValue::Int(1),
            ])),
        ),
        Some(DexExecutionResult::Void)
    );
    let field = |name: &str| crate::dex::DexFieldId {
        class_descriptor: "Landroid/graphics/RectF;".to_owned(),
        type_descriptor: "F".to_owned(),
        name: name.to_owned(),
    };

    assert_eq!(
        program.read_stored_instance_field(&bounds, &field("left")),
        Some(DexValue::Float(10.0))
    );
    assert_eq!(
        program.read_stored_instance_field(&bounds, &field("top")),
        Some(DexValue::Float(5.0))
    );
    assert_eq!(
        program.read_stored_instance_field(&bounds, &field("right")),
        Some(DexValue::Float(60.0))
    );
    assert_eq!(
        program.read_stored_instance_field(&bounds, &field("bottom")),
        Some(DexValue::Float(40.0))
    );

    let reset = build("reset", Vec::new(), "V");
    program.invoke_builtin_method(&reset, Some(&test_invocation(vec![path.clone()])));
    assert_eq!(
        program.invoke_builtin_method(&is_empty, Some(&test_invocation(vec![path]))),
        Some(DexExecutionResult::Int(1))
    );
}
